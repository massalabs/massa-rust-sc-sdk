//! Minimal in-memory implementation of the massa-sc-runtime `Interface`,
//! good enough to execute real contracts locally (ledger, datastore, calls,
//! events, coins, op-datastore, hashes, deferred calls bookkeeping).

#[cfg(feature = "clayer")]
use clayer_proto_rs as massa_proto_rs;
#[cfg(feature = "clayer")]
use clayer_sc_runtime as runtime;
use massa_proto_rs::massa::model::v1::{AddressCategory, ComparisonResult, NativeAmount, NativeTime, Slot};
#[cfg(feature = "massa")]
use massa_sc_runtime as runtime;
use runtime::{CondomLimits, GasCosts, Interface, InterfaceClone, InterfaceError, RuntimeModule};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, InterfaceError>;

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(InterfaceError::GenericError(msg.into()))
}

#[derive(Default, Clone, Debug)]
pub struct Account {
    pub balance: u64,
    pub bytecode: Vec<u8>,
    pub datastore: BTreeMap<Vec<u8>, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub address: String,
    pub coins: u64,
    pub owned: Vec<String>,
}

#[derive(Debug)]
pub struct State {
    pub ledger: BTreeMap<String, Account>,
    pub stack: Vec<CallFrame>,
    pub events: Vec<String>,
    pub prints: Vec<String>,
    pub op_datastore: BTreeMap<Vec<u8>, Vec<u8>>,
    pub time: u64,
    pub period: u64,
    pub thread: u8,
    pub chain_id: u64,
    pub interface_version: u32,
    pub sc_counter: u64,
    pub origin_op_id: Option<String>,
    pub deferred_calls: BTreeMap<String, bool>,
    pub async_messages: Vec<String>,
    pub recursion: u32,
    pub gas_costs: GasCosts,
    pub condom_limits: CondomLimits,
    pub write_access: BTreeSet<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            ledger: BTreeMap::new(),
            stack: Vec::new(),
            events: Vec::new(),
            prints: Vec::new(),
            op_datastore: BTreeMap::new(),
            time: 1_700_000_000_000,
            period: 10,
            thread: 3,
            chain_id: 77,
            interface_version: 2,
            sc_counter: 0,
            origin_op_id: Some("O1dummyOperationId".to_string()),
            deferred_calls: BTreeMap::new(),
            async_messages: Vec::new(),
            recursion: 0,
            gas_costs: GasCosts::default(),
            condom_limits: CondomLimits::default(),
            write_access: BTreeSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct SimInterface {
    pub state: Arc<Mutex<State>>,
}

impl SimInterface {
    pub fn new(state: State) -> Self {
        Self { state: Arc::new(Mutex::new(state)) }
    }
    fn st(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
    fn current(&self) -> Result<String> {
        match self.st().stack.last() {
            Some(f) => Ok(f.address.clone()),
            None => err("empty call stack"),
        }
    }
    fn has_write_access(&self, st: &State, address: &str) -> bool {
        st.stack.last().map(|f| f.address == address || f.owned.iter().any(|a| a == address)).unwrap_or(false)
            || st.write_access.contains(address)
    }
}

impl InterfaceClone for SimInterface {
    fn clone_box(&self) -> Box<dyn Interface> {
        Box::new(self.clone())
    }
}

fn hash_for_address(seed: &[u8]) -> String {
    let h = blake3::hash(seed);
    let mut v = vec![0u8]; // version 0
    v.extend_from_slice(h.as_bytes());
    format!("AS{}", bs58_check(&v))
}

fn bs58_check(data: &[u8]) -> String {
    // minimal base58check (same alphabet as bitcoin) using sha256d checksum
    let mut payload = data.to_vec();
    let c1 = sha2::Sha256::digest(&payload);
    let c2 = sha2::Sha256::digest(c1);
    payload.extend_from_slice(&c2[..4]);
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut digits: Vec<u8> = vec![0];
    for &byte in &payload {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut s = String::new();
    for &b in payload.iter().take_while(|&&b| b == 0) {
        let _ = b;
        s.push('1');
    }
    for &d in digits.iter().rev() {
        s.push(ALPHABET[d as usize] as char);
    }
    s
}

impl Interface for SimInterface {
    fn increment_recursion_counter(&self) -> Result<()> {
        let mut st = self.st();
        st.recursion += 1;
        if st.recursion > 25 {
            return Err(InterfaceError::DepthError("max recursion".into()));
        }
        Ok(())
    }
    fn decrement_recursion_counter(&self) -> Result<()> {
        let mut st = self.st();
        st.recursion = st.recursion.saturating_sub(1);
        Ok(())
    }
    fn get_interface_version(&self) -> Result<u32> {
        Ok(self.st().interface_version)
    }
    fn init_call(&self, address: &str, raw_coins: u64) -> Result<Vec<u8>> {
        let mut st = self.st();
        let from = match st.stack.last() {
            Some(f) => f.address.clone(),
            None => return err("empty stack"),
        };
        let bytecode = match st.ledger.get(address) {
            Some(a) => a.bytecode.clone(),
            None => return err(format!("address {address} not found")),
        };
        if raw_coins > 0 {
            let fb = st.ledger.get(&from).map(|a| a.balance).unwrap_or(0);
            if fb < raw_coins {
                return err("insufficient balance for call coins");
            }
            st.ledger.get_mut(&from).unwrap().balance -= raw_coins;
            st.ledger.get_mut(address).unwrap().balance += raw_coins;
        }
        st.stack.push(CallFrame {
            address: address.to_string(),
            coins: raw_coins,
            owned: vec![address.to_string()],
        });
        Ok(bytecode)
    }
    fn init_call_wasmv1(&self, _address: &str, _raw_coins: NativeAmount) -> Result<Vec<u8>> {
        err("wasmv1 unsupported")
    }
    fn finish_call(&self) -> Result<()> {
        self.st().stack.pop();
        Ok(())
    }
    fn get_balance(&self) -> Result<u64> {
        let cur = self.current()?;
        Ok(self.st().ledger.get(&cur).map(|a| a.balance).unwrap_or(0))
    }
    fn get_balance_for(&self, address: &str) -> Result<u64> {
        Ok(self.st().ledger.get(address).map(|a| a.balance).unwrap_or(0))
    }
    fn get_balance_wasmv1(&self, _address: Option<String>) -> Result<NativeAmount> {
        err("wasmv1 unsupported")
    }
    fn transfer_coins(&self, to_address: &str, raw_amount: u64) -> Result<()> {
        let from = self.current()?;
        self.transfer_coins_for(&from, to_address, raw_amount)
    }
    fn transfer_coins_for(&self, from_address: &str, to_address: &str, raw_amount: u64) -> Result<()> {
        let mut st = self.st();
        if !self.has_write_access(&st, from_address) {
            return err(format!("no write access on {from_address}"));
        }
        let fb = st.ledger.get(from_address).map(|a| a.balance).unwrap_or(0);
        if fb < raw_amount {
            return err(format!("insufficient balance: {fb} < {raw_amount}"));
        }
        st.ledger.entry(from_address.to_string()).or_default().balance -= raw_amount;
        st.ledger.entry(to_address.to_string()).or_default().balance += raw_amount;
        Ok(())
    }
    fn transfer_coins_wasmv1(&self, _t: String, _a: NativeAmount, _f: Option<String>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn get_call_coins(&self) -> Result<u64> {
        Ok(self.st().stack.last().map(|f| f.coins).unwrap_or(0))
    }
    fn get_call_coins_wasmv1(&self) -> Result<NativeAmount> {
        err("wasmv1 unsupported")
    }
    fn raw_set_bytecode(&self, bytecode: &[u8]) -> Result<()> {
        let cur = self.current()?;
        self.raw_set_bytecode_for(&cur, bytecode)
    }
    fn raw_set_bytecode_for(&self, address: &str, bytecode: &[u8]) -> Result<()> {
        let mut st = self.st();
        if !self.has_write_access(&st, address) {
            return err(format!("no write access on {address}"));
        }
        st.ledger.entry(address.to_string()).or_default().bytecode = bytecode.to_vec();
        Ok(())
    }
    fn set_bytecode_wasmv1(&self, _b: &[u8], _a: Option<String>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn create_module(&self, module: &[u8]) -> Result<String> {
        let mut st = self.st();
        st.sc_counter += 1;
        let addr = hash_for_address(&st.sc_counter.to_le_bytes());
        st.ledger.insert(
            addr.clone(),
            Account { balance: 0, bytecode: module.to_vec(), datastore: BTreeMap::new() },
        );
        if let Some(f) = st.stack.last_mut() {
            f.owned.push(addr.clone());
        }
        Ok(addr)
    }
    fn print(&self, message: &str) -> Result<()> {
        eprintln!("[print] {message}");
        self.st().prints.push(message.to_string());
        Ok(())
    }
    fn get_keys(&self, prefix: Option<&[u8]>) -> Result<BTreeSet<Vec<u8>>> {
        let cur = self.current()?;
        self.get_keys_for(&cur, prefix)
    }
    fn get_keys_for(&self, address: &str, prefix: Option<&[u8]>) -> Result<BTreeSet<Vec<u8>>> {
        let st = self.st();
        let acc = match st.ledger.get(address) {
            Some(a) => a,
            None => return err(format!("address {address} not found")),
        };
        Ok(acc
            .datastore
            .keys()
            .filter(|k| prefix.map(|p| k.starts_with(p)).unwrap_or(true))
            .cloned()
            .collect())
    }
    #[cfg(feature = "massa")]
    #[cfg(feature = "massa")]
    fn get_keys_paginated(
        &self,
        prefix: Option<&[u8]>,
        start_key: Option<&[u8]>,
        count: u32,
    ) -> Result<BTreeSet<Vec<u8>>> {
        let cur = self.current()?;
        self.get_keys_for_paginated(&cur, prefix, start_key, count)
    }
    #[cfg(feature = "massa")]
    fn get_keys_for_paginated(
        &self,
        address: &str,
        prefix: Option<&[u8]>,
        start_key: Option<&[u8]>,
        count: u32,
    ) -> Result<BTreeSet<Vec<u8>>> {
        let all = self.get_keys_for(address, prefix)?;
        Ok(all
            .into_iter()
            .filter(|k| start_key.map(|s| k.as_slice() > s).unwrap_or(true))
            .take(count as usize)
            .collect())
    }
    fn get_ds_keys_wasmv1(&self, _p: &[u8], _a: Option<String>) -> Result<BTreeSet<Vec<u8>>> {
        err("wasmv1 unsupported")
    }
    fn raw_get_data(&self, key: &[u8]) -> Result<Vec<u8>> {
        let cur = self.current()?;
        self.raw_get_data_for(&cur, key)
    }
    fn raw_get_data_for(&self, address: &str, key: &[u8]) -> Result<Vec<u8>> {
        let st = self.st();
        match st.ledger.get(address).and_then(|a| a.datastore.get(key)) {
            Some(v) => Ok(v.clone()),
            None => err(format!("datastore entry not found for key {key:?} at {address}")),
        }
    }
    fn get_ds_value_wasmv1(&self, _k: &[u8], _a: Option<String>) -> Result<Vec<u8>> {
        err("wasmv1 unsupported")
    }
    fn raw_set_data(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cur = self.current()?;
        self.raw_set_data_for(&cur, key, value)
    }
    fn raw_set_data_for(&self, address: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let mut st = self.st();
        if !self.has_write_access(&st, address) {
            return err(format!("no write access on {address}"));
        }
        if key.is_empty() || key.len() > 255 {
            return err("invalid key length");
        }
        st.ledger.entry(address.to_string()).or_default().datastore.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
    fn set_ds_value_wasmv1(&self, _k: &[u8], _v: &[u8], _a: Option<String>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn raw_append_data(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cur = self.current()?;
        self.raw_append_data_for(&cur, key, value)
    }
    fn raw_append_data_for(&self, address: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let mut st = self.st();
        if !self.has_write_access(&st, address) {
            return err(format!("no write access on {address}"));
        }
        match st.ledger.get_mut(address).and_then(|a| a.datastore.get_mut(key)) {
            Some(v) => {
                v.extend_from_slice(value);
                Ok(())
            }
            None => err("datastore entry not found"),
        }
    }
    fn append_ds_value_wasmv1(&self, _k: &[u8], _v: &[u8], _a: Option<String>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn raw_delete_data(&self, key: &[u8]) -> Result<()> {
        let cur = self.current()?;
        self.raw_delete_data_for(&cur, key)
    }
    fn raw_delete_data_for(&self, address: &str, key: &[u8]) -> Result<()> {
        let mut st = self.st();
        if !self.has_write_access(&st, address) {
            return err(format!("no write access on {address}"));
        }
        match st.ledger.get_mut(address).map(|a| a.datastore.remove(key)) {
            Some(Some(_)) => Ok(()),
            _ => err("datastore entry not found"),
        }
    }
    fn delete_ds_entry_wasmv1(&self, _k: &[u8], _a: Option<String>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn has_data(&self, key: &[u8]) -> Result<bool> {
        let cur = self.current()?;
        self.has_data_for(&cur, key)
    }
    fn has_data_for(&self, address: &str, key: &[u8]) -> Result<bool> {
        Ok(self.st().ledger.get(address).map(|a| a.datastore.contains_key(key)).unwrap_or(false))
    }
    fn ds_entry_exists_wasmv1(&self, _k: &[u8], _a: Option<String>) -> Result<bool> {
        err("wasmv1 unsupported")
    }
    fn raw_get_bytecode(&self) -> Result<Vec<u8>> {
        let cur = self.current()?;
        self.raw_get_bytecode_for(&cur)
    }
    fn raw_get_bytecode_for(&self, address: &str) -> Result<Vec<u8>> {
        match self.st().ledger.get(address) {
            Some(a) => Ok(a.bytecode.clone()),
            None => err(format!("address {address} not found")),
        }
    }
    fn get_bytecode_wasmv1(&self, _a: Option<String>) -> Result<Vec<u8>> {
        err("wasmv1 unsupported")
    }
    fn get_op_keys(&self, prefix: Option<&[u8]>) -> Result<Vec<Vec<u8>>> {
        Ok(self
            .st()
            .op_datastore
            .keys()
            .filter(|k| prefix.map(|p| k.starts_with(p)).unwrap_or(true))
            .cloned()
            .collect())
    }
    fn get_op_keys_wasmv1(&self, _p: &[u8]) -> Result<Vec<Vec<u8>>> {
        err("wasmv1 unsupported")
    }
    fn op_entry_exists(&self, key: &[u8]) -> Result<bool> {
        Ok(self.st().op_datastore.contains_key(key))
    }
    fn get_op_data(&self, key: &[u8]) -> Result<Vec<u8>> {
        match self.st().op_datastore.get(key) {
            Some(v) => Ok(v.clone()),
            None => err("op datastore entry not found"),
        }
    }
    fn caller_has_write_access(&self) -> Result<bool> {
        let st = self.st();
        let n = st.stack.len();
        if n < 2 {
            return Ok(false);
        }
        let callee = &st.stack[n - 1].address;
        let caller = &st.stack[n - 2];
        Ok(caller.owned.iter().any(|a| a == callee))
    }
    fn hash(&self, data: &[u8]) -> Result<[u8; 32]> {
        Ok(*blake3::hash(data).as_bytes())
    }
    fn hash_blake3(&self, bytes: &[u8]) -> Result<[u8; 32]> {
        Ok(*blake3::hash(bytes).as_bytes())
    }
    #[cfg(feature = "clayer")]
    fn hash_mimc(&self, bytes: &[u8]) -> Result<[u8; 32]> {
        crate::mimc::hash(bytes).map_err(InterfaceError::GenericError)
    }
    fn signature_verify(&self, _d: &[u8], _s: &str, _p: &str) -> Result<bool> {
        Ok(false)
    }
    fn evm_signature_verify(&self, _m: &[u8], _s: &[u8], _p: &[u8]) -> Result<bool> {
        Ok(false)
    }
    fn evm_get_address_from_pubkey(&self, public_key: &[u8]) -> Result<Vec<u8>> {
        let h = sha3::Keccak256::digest(public_key);
        Ok(h[12..].to_vec())
    }
    fn evm_get_pubkey_from_signature(&self, _h: &[u8], _s: &[u8]) -> Result<Vec<u8>> {
        Ok(vec![0u8; 64])
    }
    fn is_address_eoa(&self, address: &str) -> Result<bool> {
        Ok(address.starts_with("AU"))
    }
    fn address_from_public_key(&self, public_key: &str) -> Result<String> {
        Ok(format!("AU{}", &public_key[1..]))
    }
    fn validate_address(&self, address: &str) -> Result<bool> {
        Ok((address.starts_with("AU") || address.starts_with("AS")) && address.len() > 40)
    }
    fn get_time(&self) -> Result<u64> {
        Ok(self.st().time)
    }
    fn unsafe_random(&self) -> Result<i64> {
        Ok(4)
    }
    fn unsafe_random_f64(&self) -> Result<f64> {
        Ok(0.4)
    }
    fn unsafe_random_wasmv1(&self, _n: u64) -> Result<Vec<u8>> {
        err("wasmv1 unsupported")
    }
    fn get_current_period(&self) -> Result<u64> {
        Ok(self.st().period)
    }
    fn get_current_thread(&self) -> Result<u8> {
        Ok(self.st().thread)
    }
    fn get_current_slot(&self) -> Result<Slot> {
        let st = self.st();
        Ok(Slot { period: st.period, thread: st.thread as u32 })
    }
    fn get_owned_addresses(&self) -> Result<Vec<String>> {
        Ok(self.st().stack.last().map(|f| f.owned.clone()).unwrap_or_default())
    }
    fn get_call_stack(&self) -> Result<Vec<String>> {
        Ok(self.st().stack.iter().map(|f| f.address.clone()).collect())
    }
    fn generate_event(&self, event: String) -> Result<()> {
        self.st().events.push(event);
        Ok(())
    }
    fn generate_event_wasmv1(&self, _e: Vec<u8>) -> Result<()> {
        err("wasmv1 unsupported")
    }
    fn get_module(&self, bytecode: &[u8], _gas_limit: u64) -> Result<RuntimeModule> {
        let st = self.st();
        RuntimeModule::new(bytecode, st.gas_costs.clone(), runtime::Compiler::CL, st.condom_limits.clone())
            .map_err(|e| InterfaceError::GenericError(e.to_string()))
    }
    fn get_tmp_module(&self, bytecode: &[u8], _gas_limit: u64) -> Result<RuntimeModule> {
        let st = self.st();
        RuntimeModule::new(bytecode, st.gas_costs.clone(), runtime::Compiler::SP, st.condom_limits.clone())
            .map_err(|e| InterfaceError::GenericError(e.to_string()))
    }
    fn send_message(
        &self,
        target_address: &str,
        target_handler: &str,
        validity_start: (u64, u8),
        validity_end: (u64, u8),
        max_gas: u64,
        raw_fee: u64,
        raw_coins: u64,
        data: &[u8],
        filter: Option<(&str, Option<&[u8]>)>,
    ) -> Result<()> {
        self.st().async_messages.push(format!(
            "{target_address}::{target_handler} start={validity_start:?} end={validity_end:?} gas={max_gas} fee={raw_fee} coins={raw_coins} data={data:?} filter={filter:?}"
        ));
        Ok(())
    }
    fn get_origin_operation_id(&self) -> Result<Option<String>> {
        Ok(self.st().origin_op_id.clone())
    }
    fn hash_sha256(&self, bytes: &[u8]) -> Result<[u8; 32]> {
        Ok(sha2::Sha256::digest(bytes).into())
    }
    fn hash_keccak256(&self, bytes: &[u8]) -> Result<[u8; 32]> {
        Ok(sha3::Keccak256::digest(bytes).into())
    }
    fn chain_id(&self) -> Result<u64> {
        Ok(self.st().chain_id)
    }
    fn get_deferred_call_quote(
        &self,
        _slot: (u64, u8),
        gas_limit: u64,
        params_size: u64,
    ) -> Result<(bool, u64)> {
        Ok((true, 1000 + gas_limit / 100 + params_size))
    }
    fn deferred_call_register(
        &self,
        target_addr: &str,
        target_func: &str,
        target_slot: (u64, u8),
        _max_gas: u64,
        _params: &[u8],
        _coins: u64,
    ) -> Result<String> {
        let mut st = self.st();
        let id = format!("D1{}_{}_{}_{}", target_addr.len(), target_func, target_slot.0, target_slot.1);
        st.deferred_calls.insert(id.clone(), true);
        Ok(id)
    }
    fn deferred_call_exists(&self, id: &str) -> Result<bool> {
        Ok(self.st().deferred_calls.get(id).copied().unwrap_or(false))
    }
    fn deferred_call_cancel(&self, id: &str) -> Result<()> {
        let mut st = self.st();
        match st.deferred_calls.get_mut(id) {
            Some(v) => {
                *v = false;
                Ok(())
            }
            None => err("deferred call not found"),
        }
    }
    fn native_amount_from_str_wasmv1(&self, _a: &str) -> Result<NativeAmount> {
        err("wasmv1")
    }
    fn native_amount_to_string_wasmv1(&self, _a: &NativeAmount) -> Result<String> {
        err("wasmv1")
    }
    fn check_native_amount_wasmv1(&self, _a: &NativeAmount) -> Result<bool> {
        err("wasmv1")
    }
    fn add_native_amount_wasmv1(&self, _a: &NativeAmount, _b: &NativeAmount) -> Result<NativeAmount> {
        err("wasmv1")
    }
    fn sub_native_amount_wasmv1(&self, _a: &NativeAmount, _b: &NativeAmount) -> Result<NativeAmount> {
        err("wasmv1")
    }
    fn scalar_mul_native_amount_wasmv1(&self, _a: &NativeAmount, _f: u64) -> Result<NativeAmount> {
        err("wasmv1")
    }
    fn scalar_div_rem_native_amount_wasmv1(
        &self,
        _a: &NativeAmount,
        _d: u64,
    ) -> Result<(NativeAmount, NativeAmount)> {
        err("wasmv1")
    }
    fn div_rem_native_amount_wasmv1(
        &self,
        _a: &NativeAmount,
        _d: &NativeAmount,
    ) -> Result<(u64, NativeAmount)> {
        err("wasmv1")
    }
    fn check_address_wasmv1(&self, _a: &str) -> Result<bool> {
        err("wasmv1")
    }
    fn check_pubkey_wasmv1(&self, _a: &str) -> Result<bool> {
        err("wasmv1")
    }
    fn check_signature_wasmv1(&self, _a: &str) -> Result<bool> {
        err("wasmv1")
    }
    fn get_address_category_wasmv1(&self, _a: &str) -> Result<AddressCategory> {
        err("wasmv1")
    }
    fn get_address_version_wasmv1(&self, _a: &str) -> Result<u64> {
        err("wasmv1")
    }
    fn get_pubkey_version_wasmv1(&self, _a: &str) -> Result<u64> {
        err("wasmv1")
    }
    fn get_signature_version_wasmv1(&self, _a: &str) -> Result<u64> {
        err("wasmv1")
    }
    fn checked_add_native_time_wasmv1(&self, _a: &NativeTime, _b: &NativeTime) -> Result<NativeTime> {
        err("wasmv1")
    }
    fn checked_sub_native_time_wasmv1(&self, _a: &NativeTime, _b: &NativeTime) -> Result<NativeTime> {
        err("wasmv1")
    }
    fn checked_mul_native_time_wasmv1(&self, _a: &NativeTime, _f: u64) -> Result<NativeTime> {
        err("wasmv1")
    }
    fn checked_scalar_div_native_time_wasmv1(
        &self,
        _a: &NativeTime,
        _d: u64,
    ) -> Result<(NativeTime, NativeTime)> {
        err("wasmv1")
    }
    fn checked_div_native_time_wasmv1(&self, _a: &NativeTime, _d: &NativeTime) -> Result<(u64, NativeTime)> {
        err("wasmv1")
    }
    fn base58_check_to_bytes_wasmv1(&self, _s: &str) -> Result<Vec<u8>> {
        err("wasmv1")
    }
    fn bytes_to_base58_check_wasmv1(&self, bytes: &[u8]) -> String {
        bs58_check(bytes)
    }
    fn compare_address_wasmv1(&self, _l: &str, _r: &str) -> Result<ComparisonResult> {
        err("wasmv1")
    }
    fn compare_native_amount_wasmv1(&self, _l: &NativeAmount, _r: &NativeAmount) -> Result<ComparisonResult> {
        err("wasmv1")
    }
    fn compare_native_time_wasmv1(&self, _l: &NativeTime, _r: &NativeTime) -> Result<ComparisonResult> {
        err("wasmv1")
    }
    fn compare_pub_key_wasmv1(&self, _l: &str, _r: &str) -> Result<ComparisonResult> {
        err("wasmv1")
    }
    fn save_gas_remaining_before_subexecution(&self, _gas_used_until: u64) {}
}
