//! Run a compiled Massa smart contract locally, inside the real Massa runtime
//! (`massa-sc-runtime`: wasmer 4.3.6, gas metering, node limits), against an
//! in-memory ledger. Prints return values, events, gas and the final state.
//!
//! ```text
//! massa-sc-runner build/counter.wasm constructor --args u64:10 --deploy \; increment --args u64:5 \; get
//! ```

mod mimc;
mod sim;

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use runtime::{run_function, run_main, CondomLimits, GasCosts, RuntimeModule};
use sim::{Account, CallFrame, SimInterface, State};

#[cfg(feature = "clayer")]
use clayer_sc_runtime as runtime;
#[cfg(feature = "massa")]
use massa_sc_runtime as runtime;

/// Default originator / caller.
pub const CALLER: &str = "AU1dQZLc618t4CNYWYiDnFnq585kifefr1EAcMi5jddJZGQJXJXL";
/// Address at which the module under test is deployed.
pub const SC: &str = "AS12FuajZtFN2JTcbDM7KuxVoDXMvkZTKFggjPMg2Lz1PTZN8aEEy";

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

fn parse_hex(s: &str) -> Result<Vec<u8>> {
    let s = s.trim_start_matches("0x");
    if s.len() % 2 != 0 {
        bail!("odd hex length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| anyhow!("bad hex: {e}")))
        .collect()
}

/// Encode a comma-separated list of typed values with the `Args` layout:
/// `s:text`, `addr:AU...`, `bool:true`, `u8:1`, `u16:`, `u32:`, `u64:`,
/// `i32:`, `i64:`, `u128:`, `u256:`, `f64:`, `bytes:<hex>`, `hex:<raw hex appended as is>`.
fn encode_args(spec: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    if spec.trim().is_empty() {
        return Ok(out);
    }
    for item in spec.split(',') {
        let (ty, val) =
            item.split_once(':').ok_or_else(|| anyhow!("argument `{item}` must be `type:value`"))?;
        match ty.trim() {
            "s" | "str" | "string" | "addr" | "address" => {
                out.extend_from_slice(&(val.len() as u32).to_le_bytes());
                out.extend_from_slice(val.as_bytes());
            }
            "bool" => out.push(matches!(val, "true" | "1") as u8),
            "u8" => out.push(val.parse::<u8>()?),
            "i8" => out.push(val.parse::<i8>()? as u8),
            "u16" => out.extend_from_slice(&val.parse::<u16>()?.to_le_bytes()),
            "i16" => out.extend_from_slice(&val.parse::<i16>()?.to_le_bytes()),
            "u32" => out.extend_from_slice(&val.parse::<u32>()?.to_le_bytes()),
            "i32" => out.extend_from_slice(&val.parse::<i32>()?.to_le_bytes()),
            "u64" => out.extend_from_slice(&val.parse::<u64>()?.to_le_bytes()),
            "i64" => out.extend_from_slice(&val.parse::<i64>()?.to_le_bytes()),
            "u128" => out.extend_from_slice(&val.parse::<u128>()?.to_le_bytes()),
            "i128" => out.extend_from_slice(&val.parse::<i128>()?.to_le_bytes()),
            "u256" => out.extend_from_slice(&val.parse::<ethnum::U256>()?.to_le_bytes()),
            "f32" => out.extend_from_slice(&val.parse::<f32>()?.to_le_bytes()),
            "f64" => out.extend_from_slice(&val.parse::<f64>()?.to_le_bytes()),
            "bytes" => {
                let b = parse_hex(val)?;
                out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                out.extend_from_slice(&b);
            }
            "hex" => out.extend_from_slice(&parse_hex(val)?),
            other => bail!("unknown argument type `{other}`"),
        }
    }
    Ok(out)
}

fn parse_value(s: &str) -> Result<Vec<u8>> {
    if let Some(rest) = s.strip_prefix("s:") {
        Ok(rest.as_bytes().to_vec())
    } else if let Some(rest) = s.strip_prefix("hex:") {
        parse_hex(rest)
    } else if let Some(rest) = s.strip_prefix("args:") {
        encode_args(rest)
    } else if let Some(rest) = s.strip_prefix('@') {
        std::fs::read(rest).with_context(|| format!("reading {rest}"))
    } else {
        parse_hex(s)
    }
}

struct Call {
    function: String,
    param: Vec<u8>,
    coins: u64,
    deploy: bool,
    caller: String,
}

const USAGE: &str = "\
usage: massa-sc-runner <module.wasm> [global options] <function> [call options] [\\; <function> [call options] ...]

  function `main` is executed as an ExecuteSC operation by the caller.

global options:
  --chain-id <N>             chain id reported to the contract (default 77, sandbox)
  --opds <key>=<value>       operation datastore entry (values: hex, s:text, hex:.., args:<spec>, @file)
  --ds <key>=<value>         initial datastore entry of the contract
  --balance <nanoMAS>        initial balance of the contract (default 0)
  --gas <N>                  gas limit per call (default 1_000_000_000)
  --quiet                    do not print the final state

call options:
  --args <spec>              typed arguments: s:text,u64:42,u256:1000,bool:true,bytes:<hex>,addr:AU..
  --hex <hex>                raw serialized arguments
  --coins <nanoMAS>          coins sent with the call
  --caller <address>         caller address (default a funded AU address)
  --deploy                   caller has write access to the contract (deployment context)
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] == "-h" || args[1] == "--help" {
        eprint!("{USAGE}");
        std::process::exit(2);
    }
    let wasm = std::fs::read(&args[1]).with_context(|| format!("reading {}", args[1]))?;
    let mut chain_id = 77u64;
    let mut op_ds = BTreeMap::new();
    let mut initial_ds: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut sc_balance = 0u64;
    let mut gas_limit = 1_000_000_000u64;
    let mut quiet = false;
    let mut calls: Vec<Call> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: usize| -> Result<&String> {
            args.get(i + 1).ok_or_else(|| anyhow!("missing value after {a}"))
        };
        match a {
            "--chain-id" => {
                chain_id = next(i)?.parse()?;
                i += 2;
            }
            "--gas" => {
                gas_limit = next(i)?.parse()?;
                i += 2;
            }
            "--balance" => {
                sc_balance = next(i)?.parse()?;
                i += 2;
            }
            "--quiet" => {
                quiet = true;
                i += 1;
            }
            "--opds" | "--ds" => {
                let (k, v) = next(i)?.split_once('=').ok_or_else(|| anyhow!("{a} expects key=value"))?;
                let entry = (parse_value(k)?, parse_value(v)?);
                if a == "--opds" {
                    op_ds.insert(entry.0, entry.1);
                } else {
                    initial_ds.push(entry);
                }
                i += 2;
            }
            "--args" => {
                calls.last_mut().ok_or_else(|| anyhow!("--args before any function"))?.param =
                    encode_args(next(i)?)?;
                i += 2;
            }
            "--hex" => {
                calls.last_mut().ok_or_else(|| anyhow!("--hex before any function"))?.param =
                    parse_hex(next(i)?)?;
                i += 2;
            }
            "--coins" => {
                calls.last_mut().ok_or_else(|| anyhow!("--coins before any function"))?.coins =
                    next(i)?.parse()?;
                i += 2;
            }
            "--caller" => {
                calls.last_mut().ok_or_else(|| anyhow!("--caller before any function"))?.caller =
                    next(i)?.clone();
                i += 2;
            }
            "--deploy" => {
                calls.last_mut().ok_or_else(|| anyhow!("--deploy before any function"))?.deploy = true;
                i += 1;
            }
            ";" => i += 1,
            other if other.starts_with("--") => bail!("unknown option {other}"),
            other => {
                calls.push(Call {
                    function: other.to_string(),
                    param: Vec::new(),
                    coins: 0,
                    deploy: false,
                    caller: CALLER.to_string(),
                });
                i += 1;
            }
        }
    }
    if calls.is_empty() {
        bail!("no function to call");
    }

    let gas_costs = GasCosts::default();
    let limits = node_condom_limits();
    let mut state = State::default();
    state.chain_id = chain_id;
    state.gas_costs = gas_costs.clone();
    state.condom_limits = limits.clone();
    state.op_datastore = op_ds;
    state.ledger.insert(
        CALLER.to_string(),
        Account { balance: 1_000_000_000_000, bytecode: vec![], datastore: BTreeMap::new() },
    );
    state.ledger.insert(
        SC.to_string(),
        Account { balance: sc_balance, bytecode: wasm.clone(), datastore: initial_ds.into_iter().collect() },
    );
    let interface = SimInterface::new(state);

    let t0 = std::time::Instant::now();
    let module = RuntimeModule::new(&wasm, gas_costs.clone(), runtime::Compiler::CL, limits.clone())
        .map_err(|e| anyhow!("MODULE REJECTED BY THE RUNTIME: {e:#}"))?;
    eprintln!("module compiled in {:.1?}", t0.elapsed());

    let mut failed = false;
    for call in calls {
        let is_main = call.function == "main";
        {
            let mut st = interface.state.lock().unwrap();
            st.stack.clear();
            st.ledger.entry(call.caller.clone()).or_insert(Account {
                balance: 1_000_000_000_000,
                bytecode: vec![],
                datastore: BTreeMap::new(),
            });
            if is_main {
                st.ledger.get_mut(&call.caller).unwrap().bytecode = wasm.clone();
                st.stack.push(CallFrame {
                    address: call.caller.clone(),
                    coins: 0,
                    owned: vec![call.caller.clone()],
                });
            } else {
                if call.coins > 0 {
                    let b = &mut st.ledger.get_mut(&call.caller).unwrap().balance;
                    *b =
                        b.checked_sub(call.coins).ok_or_else(|| anyhow!("caller cannot afford the coins"))?;
                    st.ledger.get_mut(SC).unwrap().balance += call.coins;
                }
                let mut owned = vec![call.caller.clone()];
                if call.deploy {
                    owned.push(SC.to_string());
                }
                st.stack.push(CallFrame { address: call.caller.clone(), coins: 0, owned });
                st.stack.push(CallFrame {
                    address: SC.to_string(),
                    coins: call.coins,
                    owned: vec![SC.to_string()],
                });
            }
        }
        println!("== {}({}) coins={} caller={}", call.function, hex(&call.param), call.coins, call.caller);
        let t = std::time::Instant::now();
        let res = if is_main {
            run_main(&interface, module.clone(), gas_limit, gas_costs.clone(), limits.clone())
        } else {
            run_function(
                &interface,
                module.clone(),
                &call.function,
                &call.param,
                gas_limit,
                gas_costs.clone(),
                limits.clone(),
            )
        };
        let mut st = interface.state.lock().unwrap();
        match res {
            Ok(resp) => {
                println!(
                    "   ok: return={} {:?} gas={} ({:.1?})",
                    hex(&resp.ret),
                    String::from_utf8_lossy(&resp.ret),
                    gas_limit - resp.remaining_gas,
                    t.elapsed()
                );
            }
            Err(e) => {
                failed = true;
                println!("   ERROR: {e}");
            }
        }
        for ev in st.events.drain(..) {
            println!("   event: {ev}");
        }
        for m in st.async_messages.drain(..) {
            println!("   async message: {m}");
        }
    }
    if !quiet {
        let st = interface.state.lock().unwrap();
        println!("== final state");
        for (addr, acc) in &st.ledger {
            println!(
                "{addr}: balance={} bytecode={}B datastore={} keys",
                acc.balance,
                acc.bytecode.len(),
                acc.datastore.len()
            );
            for (k, v) in &acc.datastore {
                let ks = String::from_utf8_lossy(k);
                let kd = if !k.is_empty() && ks.chars().all(|c| !c.is_control() && c != '\u{FFFD}') {
                    format!("{ks:?}")
                } else {
                    format!("0x{}", hex(k))
                };
                println!("   {kd} => {}", if v.len() <= 64 { hex(v) } else { format!("{} bytes", v.len()) });
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
