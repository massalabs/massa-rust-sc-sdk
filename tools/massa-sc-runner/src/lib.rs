//! Execute Massa smart-contract modules locally, inside the real Massa engine
//! (`massa-sc-runtime`: wasmer, gas metering, node limits), against an
//! in-memory ledger. Several contracts can be deployed and call each other,
//! which makes it a test bench for cross-contract and cross-SDK scenarios.
//!
//! ```no_run
//! use massa_sc_runner::{Runner, DEFAULT_CALLER};
//!
//! let mut runner = Runner::new();
//! let token = runner.deploy_new(&std::fs::read("build/token.wasm").unwrap(), b"", 0).unwrap();
//! let out = runner.call(&token, "balanceOf", &[]).unwrap();
//! assert!(out.events.is_empty());
//! ```

mod mimc;
mod sim;

use std::collections::BTreeMap;

#[cfg(feature = "clayer")]
use clayer_sc_runtime as runtime;
#[cfg(feature = "massa")]
use massa_sc_runtime as runtime;
use runtime::{run_function, run_main, CondomLimits, GasCosts, RuntimeModule};
use sim::{Account, CallFrame, SimInterface, State};

/// Default originator / caller, funded with 1000 MAS.
pub const DEFAULT_CALLER: &str = "AU1dQZLc618t4CNYWYiDnFnq585kifefr1EAcMi5jddJZGQJXJXL";
/// Default address of the contract under test when a single module is run.
pub const DEFAULT_CONTRACT: &str = "AS12FuajZtFN2JTcbDM7KuxVoDXMvkZTKFggjPMg2Lz1PTZN8aEEy";
/// Chain id of a sandbox node.
pub const SANDBOX_CHAIN_ID: u64 = 77;

const INITIAL_BALANCE: u64 = 1_000_000_000_000;

/// Limits applied by `massa-node` (`massa-node/src/main.rs`).
fn node_condom_limits() -> CondomLimits {
    CondomLimits {
        max_exports: Some(512 + 512),
        max_functions: Some(512 + 256),
        max_signature_len: Some(64 + 8),
        max_name_len: Some(256),
        max_imports_len: Some(256 + 1),
        max_table_initializers_len: Some(512),
        max_passive_elements_len: Some(512),
        max_passive_data_len: Some(512),
        max_global_initializers_len: Some(512),
        max_function_names_len: Some(256),
        max_tables_count: Some(16),
        max_memories_len: Some(1),
        max_globals_len: Some(512),
        max_custom_sections_len: Some(1),
        max_custom_sections_data_len: Some(1_000_000),
    }
}

/// One ledger account as reported by [`Runner::accounts`].
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub balance: u64,
    pub bytecode_len: usize,
    pub datastore: BTreeMap<Vec<u8>, Vec<u8>>,
}

/// Result of a successful execution.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Bytes returned by the entry point (empty for `main` and `void` functions).
    pub ret: Vec<u8>,
    /// Gas consumed.
    pub gas: u64,
    /// Events emitted during the execution (sub-calls included).
    pub events: Vec<String>,
    /// Asynchronous messages sent during the execution.
    pub async_messages: Vec<String>,
}

/// Options of a call.
#[derive(Debug, Clone)]
pub struct CallOptions {
    /// Calling address (default [`DEFAULT_CALLER`], created and funded when missing).
    pub caller: String,
    /// Coins sent with the call.
    pub coins: u64,
    /// Give the caller write access to the target, like right after `create_sc`
    /// (what `constructor` requires).
    pub deploy_context: bool,
}

impl Default for CallOptions {
    fn default() -> Self {
        Self { caller: DEFAULT_CALLER.to_string(), coins: 0, deploy_context: false }
    }
}

/// An in-memory Massa ledger plus the runtime to execute modules against it.
pub struct Runner {
    interface: SimInterface,
    gas_costs: GasCosts,
    limits: CondomLimits,
    /// Gas limit of each call (default 1 billion).
    pub gas_limit: u64,
    /// Print events as they are emitted.
    pub trace: bool,
    next_address: u64,
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner {
    /// A ledger with [`DEFAULT_CALLER`] funded and nothing deployed.
    pub fn new() -> Self {
        let gas_costs = GasCosts::default();
        let limits = node_condom_limits();
        let mut state = State {
            chain_id: SANDBOX_CHAIN_ID,
            gas_costs: gas_costs.clone(),
            condom_limits: limits.clone(),
            ..Default::default()
        };
        state
            .ledger
            .insert(DEFAULT_CALLER.to_string(), Account { balance: INITIAL_BALANCE, ..Default::default() });
        Self {
            interface: SimInterface::new(state),
            gas_costs,
            limits,
            gas_limit: 1_000_000_000,
            trace: false,
            next_address: 0,
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.interface.state.lock().unwrap()
    }

    /// Chain id reported to contracts (default: sandbox).
    pub fn set_chain_id(&mut self, chain_id: u64) {
        self.state().chain_id = chain_id;
    }

    /// Current slot reported to contracts.
    pub fn set_slot(&mut self, period: u64, thread: u8) {
        let mut st = self.state();
        st.period = period;
        st.thread = thread;
    }

    /// Set the balance of an address (created when missing).
    pub fn set_balance(&mut self, address: &str, balance: u64) {
        self.state().ledger.entry(address.to_string()).or_default().balance = balance;
    }

    pub fn balance(&self, address: &str) -> u64 {
        self.state().ledger.get(address).map(|a| a.balance).unwrap_or(0)
    }

    /// Set an operation datastore entry (read by `ExecuteSC` executions).
    pub fn set_op_datastore(&mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
        self.state().op_datastore.insert(key.into(), value.into());
    }

    /// Write directly into the datastore of an address.
    pub fn set_datastore(&mut self, address: &str, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
        self.state()
            .ledger
            .entry(address.to_string())
            .or_default()
            .datastore
            .insert(key.into(), value.into());
    }

    /// Snapshot of the datastore of an address.
    pub fn datastore(&self, address: &str) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.state().ledger.get(address).map(|a| a.datastore.clone()).unwrap_or_default()
    }

    pub fn bytecode(&self, address: &str) -> Vec<u8> {
        self.state().ledger.get(address).map(|a| a.bytecode.clone()).unwrap_or_default()
    }

    /// Check that `bytecode` is accepted by the runtime (feature set, limits...).
    pub fn check_module(&self, bytecode: &[u8]) -> Result<(), String> {
        RuntimeModule::new(bytecode, self.gas_costs.clone(), runtime::Compiler::CL, self.limits.clone())
            .map(|_| ())
            .map_err(|e| format!("module rejected by the runtime: {e:#}"))
    }

    /// Store `bytecode` at `address` (created when missing), keeping its
    /// datastore and balance: a deployment without constructor call, or a
    /// bytecode upgrade.
    pub fn set_bytecode(&mut self, address: &str, bytecode: &[u8]) -> Result<(), String> {
        self.check_module(bytecode)?;
        self.state().ledger.entry(address.to_string()).or_default().bytecode = bytecode.to_vec();
        Ok(())
    }

    /// A fresh, deterministic, valid-looking contract address.
    pub fn new_address(&mut self) -> String {
        self.next_address += 1;
        sim::generated_address(self.next_address)
    }

    /// Deploy `bytecode` at `address` and, when the module exports
    /// `constructor`, call it with `constructor_args` and `coins` from
    /// [`DEFAULT_CALLER`] with write access, like the standard deployer does.
    pub fn deploy(
        &mut self,
        address: &str,
        bytecode: &[u8],
        constructor_args: &[u8],
        coins: u64,
    ) -> Result<Option<Outcome>, String> {
        self.set_bytecode(address, bytecode)?;
        if self.function_exists(bytecode, "constructor")? {
            let opts = CallOptions { coins, deploy_context: true, ..Default::default() };
            return self.call_with(address, "constructor", constructor_args, opts).map(Some);
        }
        Ok(None)
    }

    /// [`deploy`](Self::deploy) at a new address; returns it.
    pub fn deploy_new(
        &mut self,
        bytecode: &[u8],
        constructor_args: &[u8],
        coins: u64,
    ) -> Result<String, String> {
        let address = self.new_address();
        self.deploy(&address, bytecode, constructor_args, coins)?;
        Ok(address)
    }

    /// Whether the module exports `function` (export section scan).
    fn function_exists(&self, bytecode: &[u8], function: &str) -> Result<bool, String> {
        use wasmparser::{ExternalKind, Parser, Payload};
        for payload in Parser::new(0).parse_all(bytecode) {
            if let Payload::ExportSection(reader) = payload.map_err(|e| format!("malformed wasm: {e}"))? {
                for export in reader {
                    let export = export.map_err(|e| format!("malformed wasm: {e}"))?;
                    if export.kind == ExternalKind::Func && export.name == function {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Call `function` of the contract at `target` from [`DEFAULT_CALLER`].
    pub fn call(&mut self, target: &str, function: &str, param: impl AsRef<[u8]>) -> Result<Outcome, String> {
        self.call_with(target, function, param, CallOptions::default())
    }

    /// Call `function` of the contract at `target` from `caller`, sending `coins`.
    pub fn call_from(
        &mut self,
        caller: &str,
        target: &str,
        function: &str,
        param: impl AsRef<[u8]>,
        coins: u64,
    ) -> Result<Outcome, String> {
        let opts = CallOptions { caller: caller.to_string(), coins, deploy_context: false };
        self.call_with(target, function, param, opts)
    }

    /// Call with every option. Returns the runtime error message when the
    /// execution fails; state changes of a failed call are NOT rolled back
    /// (this is a development tool, not a node).
    pub fn call_with(
        &mut self,
        target: &str,
        function: &str,
        param: impl AsRef<[u8]>,
        opts: CallOptions,
    ) -> Result<Outcome, String> {
        let bytecode = {
            let mut st = self.state();
            st.stack.clear();
            st.events.clear();
            st.async_messages.clear();
            st.ledger
                .entry(opts.caller.clone())
                .or_insert_with(|| Account { balance: INITIAL_BALANCE, ..Default::default() });
            let bytecode = match st.ledger.get(target) {
                Some(a) if !a.bytecode.is_empty() => a.bytecode.clone(),
                _ => return Err(format!("no contract deployed at {target}")),
            };
            if opts.coins > 0 {
                let caller_balance = &mut st.ledger.get_mut(&opts.caller).unwrap().balance;
                *caller_balance = caller_balance
                    .checked_sub(opts.coins)
                    .ok_or_else(|| format!("{} cannot afford {} coins", opts.caller, opts.coins))?;
                st.ledger.get_mut(target).unwrap().balance += opts.coins;
            }
            let mut owned = vec![opts.caller.clone()];
            if opts.deploy_context {
                owned.push(target.to_string());
            }
            st.stack.push(CallFrame { address: opts.caller.clone(), coins: 0, owned });
            st.stack.push(CallFrame {
                address: target.to_string(),
                coins: opts.coins,
                owned: vec![target.to_string()],
            });
            bytecode
        };
        let module =
            RuntimeModule::new(&bytecode, self.gas_costs.clone(), runtime::Compiler::CL, self.limits.clone())
                .map_err(|e| format!("module rejected by the runtime: {e:#}"))?;
        let res = run_function(
            &self.interface,
            module,
            function,
            param.as_ref(),
            self.gas_limit,
            self.gas_costs.clone(),
            self.limits.clone(),
        );
        self.finish(res.map(|r| (r.ret, r.remaining_gas)).map_err(|e| e.to_string()))
    }

    /// Run `bytecode` as an `ExecuteSC` operation sent by `caller`: its `main`
    /// executes in the context of the caller's address.
    pub fn execute(&mut self, caller: &str, bytecode: &[u8]) -> Result<Outcome, String> {
        {
            let mut st = self.state();
            st.stack.clear();
            st.events.clear();
            st.async_messages.clear();
            let account = st
                .ledger
                .entry(caller.to_string())
                .or_insert_with(|| Account { balance: INITIAL_BALANCE, ..Default::default() });
            account.bytecode = bytecode.to_vec();
            st.stack.push(CallFrame {
                address: caller.to_string(),
                coins: 0,
                owned: vec![caller.to_string()],
            });
        }
        let module =
            RuntimeModule::new(bytecode, self.gas_costs.clone(), runtime::Compiler::CL, self.limits.clone())
                .map_err(|e| format!("module rejected by the runtime: {e:#}"))?;
        let res =
            run_main(&self.interface, module, self.gas_limit, self.gas_costs.clone(), self.limits.clone());
        self.finish(res.map(|r| (r.ret, r.remaining_gas)).map_err(|e| e.to_string()))
    }

    fn finish(&mut self, res: Result<(Vec<u8>, u64), String>) -> Result<Outcome, String> {
        let mut st = self.state();
        let events = std::mem::take(&mut st.events);
        let async_messages = std::mem::take(&mut st.async_messages);
        if self.trace {
            for e in &events {
                eprintln!("   event: {e}");
            }
        }
        match res {
            Ok((ret, remaining)) => {
                Ok(Outcome { ret, gas: self.gas_limit - remaining, events, async_messages })
            }
            Err(e) => Err(e),
        }
    }

    /// Every account of the ledger.
    pub fn accounts(&self) -> Vec<AccountInfo> {
        self.state()
            .ledger
            .iter()
            .map(|(a, acc)| AccountInfo {
                address: a.clone(),
                balance: acc.balance,
                bytecode_len: acc.bytecode.len(),
                datastore: acc.datastore.clone(),
            })
            .collect()
    }
}
