//! In-memory Massa VM used when the SDK is compiled for a native target: every
//! `env` function is served by it, so contract logic runs under plain
//! `cargo test`, no wasm or node needed (the Rust counterpart of
//! `@massalabs/massa-as-sdk/vm-mock`).
//!
//! State is thread-local: each test gets its own VM. Call [`reset`] at the
//! start of a test, then shape the context with the `set_*` helpers.
//!
//! ```
//! use massa_sc_sdk::prelude::*;
//! use massa_sc_sdk::mock;
//!
//! #[export]
//! pub fn constructor(mut args: Args) {
//!     assert!(context::is_deploying_contract());
//!     let owner: Address = args.must_next("owner");
//!     storage::set_value("owner", &owner);
//!     generate_event("OWNER_SET");
//! }
//!
//! mock::reset();
//! mock::set_deploy_context(None);          // the caller has write access
//! constructor(Args::new().add(context::caller()));
//! assert_eq!(mock::events(), vec!["OWNER_SET"]);
//! assert_eq!(storage::get_value::<Address>("owner"), context::caller());
//! ```

extern crate std;

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::string::{String, ToString};
use std::vec::Vec;

use crate::constants::{MAX_DATASTORE_KEYS_PAGE, MOCK_CHAIN_ID};
use crate::Address;

/// Default originator / caller (same as the AssemblyScript vm-mock).
pub const DEFAULT_CALLER: &str = "AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq";
/// Default executing contract (same as the AssemblyScript vm-mock).
pub const DEFAULT_CALLEE: &str = "AS12BqZEQ6sByhRLyEuf0YbQmcF2PsDdkNNG1akBJu9XcjZA1eT";
/// Initial balance of the two default addresses: 100 MAS.
pub const DEFAULT_BALANCE: u64 = 100 * crate::constants::ONE_MAS;

/// An account of the mock ledger.
#[derive(Clone, Debug, Default)]
pub struct MockAccount {
    /// Balance in nanoMAS.
    pub balance: u64,
    /// Deployed bytecode (empty for user accounts).
    pub bytecode: Vec<u8>,
    /// Key/value datastore.
    pub datastore: BTreeMap<Vec<u8>, Vec<u8>>,
}

/// A call made by the contract under test to another contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedCall {
    /// Target contract.
    pub address: String,
    /// Called function.
    pub function: String,
    /// Serialized arguments.
    pub args: Vec<u8>,
    /// Coins sent with the call.
    pub coins: u64,
    /// `false` for `local_call`.
    pub remote: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// An asynchronous message sent with [`crate::contract::async_call`].
pub struct AsyncMessage {
    /// Target contract.
    pub address: String,
    /// Target function.
    pub function: String,
    /// First slot of the validity window.
    pub validity_start: (u64, u8),
    /// Last slot of the validity window.
    pub validity_end: (u64, u8),
    /// Gas allocated to the execution.
    pub max_gas: u64,
    /// Priority fee.
    pub fee: u64,
    /// Coins transferred with the message.
    pub coins: u64,
    /// Serialized arguments.
    pub args: Vec<u8>,
    /// Trigger address (empty = none).
    pub filter_address: String,
    /// Trigger datastore key (empty = any change).
    pub filter_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A deferred call booked with [`crate::deferred_calls::register`].
pub struct DeferredCall {
    /// Id returned at registration.
    pub id: String,
    /// Target contract.
    pub address: String,
    /// Target function.
    pub function: String,
    /// Booked slot.
    pub slot: (u64, u8),
    /// Gas allocated to the execution.
    pub max_gas: u64,
    /// Serialized arguments.
    pub args: Vec<u8>,
    /// Coins transferred with the call.
    pub coins: u64,
    /// `false` once cancelled.
    pub scheduled: bool,
}

struct MockedCall {
    address: Option<String>,
    function: Option<String>,
    ret: Vec<u8>,
}

type CallHandler = std::boxed::Box<dyn FnMut(&str, &str, &[u8], u64) -> Option<Vec<u8>>>;

/// The mock VM state. Reach it with [`with_vm`] when the helper functions
/// below are not enough.
pub struct MockVm {
    /// Every known address.
    pub ledger: BTreeMap<String, MockAccount>,
    /// Call stack, originator first, executing contract last.
    pub call_stack: Vec<String>,
    /// Addresses created during the current execution (write access).
    pub created: Vec<String>,
    /// Events emitted so far.
    pub events: Vec<String>,
    /// Messages printed so far.
    pub prints: Vec<String>,
    /// Operation datastore.
    pub op_datastore: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Coins sent with the current call.
    pub transferred_coins: u64,
    /// Slot timestamp, milliseconds.
    pub time: u64,
    /// Current period.
    pub period: u64,
    /// Current thread.
    pub thread: u8,
    /// Chain id.
    pub chain_id: u64,
    /// Result of `caller_has_write_access`.
    pub admin_context: bool,
    /// Origin operation id (`None` = not started by an operation).
    pub origin_operation_id: Option<String>,
    /// Result of `remaining_gas`.
    pub remaining_gas: u64,
    /// Result of `unsafe_random`.
    pub random: i64,
    /// Result of Massa signature checks.
    pub signature_valid: bool,
    /// Result of EVM signature checks.
    pub evm_signature_valid: bool,
    /// Public key returned by EVM signature recovery.
    pub evm_recovered_pubkey: Vec<u8>,
    /// Result of `function_exists`.
    pub function_exists: bool,
    /// Calls made to other contracts.
    pub recorded_calls: Vec<RecordedCall>,
    /// Asynchronous messages sent.
    pub async_messages: Vec<AsyncMessage>,
    /// Deferred calls booked.
    pub deferred_calls: Vec<DeferredCall>,
    mocked_calls: VecDeque<MockedCall>,
    call_handler: Option<CallHandler>,
    sc_counter: u64,
}

impl Default for MockVm {
    fn default() -> Self {
        let mut ledger = BTreeMap::new();
        ledger.insert(
            DEFAULT_CALLER.to_string(),
            MockAccount { balance: DEFAULT_BALANCE, ..Default::default() },
        );
        ledger.insert(
            DEFAULT_CALLEE.to_string(),
            MockAccount { balance: DEFAULT_BALANCE, ..Default::default() },
        );
        Self {
            ledger,
            call_stack: std::vec![DEFAULT_CALLER.to_string(), DEFAULT_CALLEE.to_string()],
            created: Vec::new(),
            events: Vec::new(),
            prints: Vec::new(),
            op_datastore: BTreeMap::new(),
            transferred_coins: 0,
            time: 1_700_000_000_000,
            period: 0,
            thread: 1,
            chain_id: MOCK_CHAIN_ID,
            admin_context: false,
            origin_operation_id: Some("O1mock000000000000000000000000000000000000000000000".to_string()),
            remaining_gas: 1_000_000_000_000_000_000,
            random: 4,
            signature_valid: true,
            evm_signature_valid: true,
            evm_recovered_pubkey: std::vec![0u8; 64],
            function_exists: true,
            recorded_calls: Vec::new(),
            async_messages: Vec::new(),
            deferred_calls: Vec::new(),
            mocked_calls: VecDeque::new(),
            call_handler: None,
            sc_counter: 0,
        }
    }
}

std::thread_local! {
    static VM: RefCell<MockVm> = RefCell::new(MockVm::default());
}

/// Run `f` with exclusive access to the thread's VM.
pub fn with_vm<R>(f: impl FnOnce(&mut MockVm) -> R) -> R {
    VM.with(|vm| f(&mut vm.borrow_mut()))
}

#[track_caller]
fn fail(msg: impl core::fmt::Display) -> ! {
    panic!("vm-mock: {msg}")
}

fn is_valid_address(address: &str) -> bool {
    (address.starts_with("AU") || address.starts_with("AS")) && (50..=56).contains(&address.len())
}

impl MockVm {
    fn callee(&self) -> String {
        self.call_stack.last().cloned().unwrap_or_else(|| fail("empty call stack"))
    }

    fn caller(&self) -> String {
        let n = self.call_stack.len();
        if n >= 2 {
            self.call_stack[n - 2].clone()
        } else {
            self.callee()
        }
    }

    fn can_write(&self, address: &str) -> bool {
        address == self.callee() || self.created.iter().any(|a| a == address) || self.admin_context
    }

    fn account(&self, address: &str) -> &MockAccount {
        self.ledger
            .get(address)
            .unwrap_or_else(|| fail(format_args!("address {address} does not exist in the mock ledger")))
    }

    fn account_mut(&mut self, address: &str) -> &mut MockAccount {
        self.ledger.entry(address.to_string()).or_default()
    }

    fn generated_address(&mut self, prefix: &str) -> String {
        const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        self.sc_counter += 1;
        let mut n = self.sc_counter.wrapping_mul(0x9E3779B97F4A7C15);
        let mut s = String::from(prefix);
        s.push_str("12");
        for _ in 0..49 {
            s.push(ALPHABET[(n % 58) as usize] as char);
            n = n / 58 + 0x2545F491 * (self.sc_counter + 1);
        }
        s
    }

    // ---- env implementation -------------------------------------------------

    pub(crate) fn print(&mut self, message: &str) {
        std::println!("{message}");
        self.prints.push(message.to_string());
    }

    pub(crate) fn generate_event(&mut self, event: &str) {
        self.events.push(event.to_string());
    }

    /// Bookkeeping of an outgoing call (recording + coin transfer).
    fn start_call(&mut self, address: &str, function: &str, args: &[u8], coins: u64, remote: bool) {
        self.recorded_calls.push(RecordedCall {
            address: address.to_string(),
            function: function.to_string(),
            args: args.to_vec(),
            coins,
            remote,
        });
        if coins > 0 {
            let from = self.callee();
            let balance = self.account(&from).balance;
            if balance < coins {
                fail(format_args!("not enough balance to pay the call to {function}: {balance} < {coins}"));
            }
            self.account_mut(&from).balance -= coins;
            self.account_mut(address).balance += coins;
        }
    }

    fn pop_mocked_call(&mut self, address: &str, function: &str) -> Option<Vec<u8>> {
        let pos = self.mocked_calls.iter().position(|m| {
            m.address.as_deref().is_none_or(|a| a == address)
                && m.function.as_deref().is_none_or(|f| f == function)
        })?;
        Some(self.mocked_calls.remove(pos).unwrap().ret)
    }
}

/// Resolve a call to another contract: handler first, then queued mocks.
/// The handler runs without holding the VM borrow, so it may use the SDK.
fn dispatch_call(address: &str, function: &str, args: &[u8], coins: u64, remote: bool) -> Vec<u8> {
    with_vm(|vm| vm.start_call(address, function, args, coins, remote));
    if let Some(mut handler) = with_vm(|vm| vm.call_handler.take()) {
        let ret = handler(address, function, args, coins);
        with_vm(|vm| {
            if vm.call_handler.is_none() {
                vm.call_handler = Some(handler);
            }
        });
        if let Some(ret) = ret {
            return ret;
        }
    }
    match with_vm(|vm| vm.pop_mocked_call(address, function)) {
        Some(ret) => ret,
        None => fail(format_args!(
            "no mock defined for call to {address}::{function} (use mock::mock_call or mock::set_call_handler)"
        )),
    }
}

pub(crate) fn call(address: &str, function: &str, args: &[u8], coins: u64) -> Vec<u8> {
    dispatch_call(address, function, args, coins, true)
}

pub(crate) fn local_call(address: &str, function: &str, args: &[u8]) -> Vec<u8> {
    dispatch_call(address, function, args, 0, false)
}

pub(crate) fn local_execution(_bytecode: &[u8], function: &str, args: &[u8]) -> Vec<u8> {
    dispatch_call("", function, args, 0, false)
}

impl MockVm {
    pub(crate) fn create_sc(&mut self, bytecode: &[u8]) -> String {
        let address = self.generated_address("AS");
        self.ledger.insert(
            address.clone(),
            MockAccount { balance: 0, bytecode: bytecode.to_vec(), datastore: BTreeMap::new() },
        );
        self.created.push(address.clone());
        address
    }

    pub(crate) fn get_bytecode(&mut self) -> Vec<u8> {
        let callee = self.callee();
        self.account(&callee).bytecode.clone()
    }

    pub(crate) fn get_bytecode_of(&mut self, address: &str) -> Vec<u8> {
        self.account(address).bytecode.clone()
    }

    pub(crate) fn set_bytecode(&mut self, bytecode: &[u8]) {
        let callee = self.callee();
        self.account_mut(&callee).bytecode = bytecode.to_vec();
    }

    pub(crate) fn set_bytecode_of(&mut self, address: &str, bytecode: &[u8]) {
        if !self.can_write(address) {
            fail(format_args!("no write access to {address}"));
        }
        self.account_mut(address).bytecode = bytecode.to_vec();
    }

    pub(crate) fn function_exists(&mut self, _address: &str, _function: &str) -> bool {
        self.function_exists
    }

    pub(crate) fn caller_has_write_access(&mut self) -> bool {
        self.admin_context
    }

    pub(crate) fn remaining_gas(&mut self) -> u64 {
        self.remaining_gas
    }

    fn check_key(key: &[u8]) {
        if key.is_empty() || key.len() > crate::constants::MAX_DATASTORE_KEY_LENGTH {
            fail(format_args!("invalid datastore key length: {}", key.len()));
        }
    }

    pub(crate) fn set(&mut self, key: &[u8], value: &[u8]) {
        let callee = self.callee();
        self.set_of(&callee, key, value)
    }

    pub(crate) fn set_of(&mut self, address: &str, key: &[u8], value: &[u8]) {
        Self::check_key(key);
        if !self.can_write(address) {
            fail(format_args!("no write access to the datastore of {address}"));
        }
        if !self.ledger.contains_key(address) {
            fail(format_args!("address {address} does not exist"));
        }
        self.account_mut(address).datastore.insert(key.to_vec(), value.to_vec());
    }

    pub(crate) fn get(&mut self, key: &[u8]) -> Vec<u8> {
        let callee = self.callee();
        self.get_of(&callee, key)
    }

    pub(crate) fn get_of(&mut self, address: &str, key: &[u8]) -> Vec<u8> {
        match self.account(address).datastore.get(key) {
            Some(v) => v.clone(),
            None => {
                fail(format_args!("datastore key {:?} not found at {address}", String::from_utf8_lossy(key)))
            }
        }
    }

    pub(crate) fn del(&mut self, key: &[u8]) {
        let callee = self.callee();
        self.delete_of(&callee, key)
    }

    pub(crate) fn delete_of(&mut self, address: &str, key: &[u8]) {
        if !self.can_write(address) {
            fail(format_args!("no write access to the datastore of {address}"));
        }
        if self.account_mut(address).datastore.remove(key).is_none() {
            fail(format_args!("datastore key {:?} not found at {address}", String::from_utf8_lossy(key)));
        }
    }

    pub(crate) fn append(&mut self, key: &[u8], value: &[u8]) {
        let callee = self.callee();
        self.append_of(&callee, key, value)
    }

    pub(crate) fn append_of(&mut self, address: &str, key: &[u8], value: &[u8]) {
        if !self.can_write(address) {
            fail(format_args!("no write access to the datastore of {address}"));
        }
        match self.account_mut(address).datastore.get_mut(key) {
            Some(v) => v.extend_from_slice(value),
            None => {
                fail(format_args!("datastore key {:?} not found at {address}", String::from_utf8_lossy(key)))
            }
        }
    }

    pub(crate) fn has(&mut self, key: &[u8]) -> bool {
        let callee = self.callee();
        self.has_of(&callee, key)
    }

    pub(crate) fn has_of(&mut self, address: &str, key: &[u8]) -> bool {
        self.ledger.get(address).map(|a| a.datastore.contains_key(key)).unwrap_or(false)
    }

    fn keys_of_filtered(&self, address: &str, prefix: &[u8], start_key: Option<&[u8]>) -> Vec<Vec<u8>> {
        self.account(address)
            .datastore
            .keys()
            .filter(|k| k.starts_with(prefix))
            .filter(|k| start_key.is_none_or(|s| k.as_slice() > s))
            .cloned()
            .collect()
    }

    pub(crate) fn get_keys(&mut self, prefix: &[u8]) -> Vec<Vec<u8>> {
        let callee = self.callee();
        self.get_keys_of(&callee, prefix)
    }

    pub(crate) fn get_keys_of(&mut self, address: &str, prefix: &[u8]) -> Vec<Vec<u8>> {
        let keys = self.keys_of_filtered(address, prefix, None);
        if keys.len() > MAX_DATASTORE_KEYS_PAGE as usize {
            fail(format_args!(
                "get_keys matched {} keys, more than MAX_DATASTORE_KEYS_PAGE ({MAX_DATASTORE_KEYS_PAGE}): use get_keys_page",
                keys.len()
            ));
        }
        keys
    }

    pub(crate) fn get_keys_page(&mut self, prefix: &[u8], start_key: &[u8], count: u32) -> Vec<Vec<u8>> {
        let callee = self.callee();
        self.get_keys_of_page(&callee, prefix, start_key, count)
    }

    pub(crate) fn get_keys_of_page(
        &mut self,
        address: &str,
        prefix: &[u8],
        start_key: &[u8],
        count: u32,
    ) -> Vec<Vec<u8>> {
        if !(1..=MAX_DATASTORE_KEYS_PAGE).contains(&count) {
            fail(format_args!(
                "datastore key page size must be between 1 and {MAX_DATASTORE_KEYS_PAGE}, got {count}"
            ));
        }
        let start = if start_key.is_empty() { None } else { Some(start_key) };
        let mut keys = self.keys_of_filtered(address, prefix, start);
        keys.truncate(count as usize);
        keys
    }

    pub(crate) fn owned_addresses(&mut self) -> Vec<String> {
        let mut v = std::vec![self.callee()];
        v.extend(self.created.iter().cloned());
        v
    }

    pub(crate) fn call_stack(&mut self) -> Vec<String> {
        self.call_stack.clone()
    }

    pub(crate) fn call_coins(&mut self) -> u64 {
        self.transferred_coins
    }

    pub(crate) fn time(&mut self) -> u64 {
        self.time
    }

    pub(crate) fn current_period(&mut self) -> u64 {
        self.period
    }

    pub(crate) fn current_thread(&mut self) -> u8 {
        self.thread
    }

    pub(crate) fn origin_operation_id(&mut self) -> String {
        self.origin_operation_id.clone().unwrap_or_default()
    }

    pub(crate) fn chain_id(&mut self) -> u64 {
        self.chain_id
    }

    pub(crate) fn unsafe_random(&mut self) -> i64 {
        self.random
    }

    pub(crate) fn transfer_coins(&mut self, to: &str, amount: u64) {
        let from = self.callee();
        self.transfer_coins_of(&from, to, amount)
    }

    pub(crate) fn transfer_coins_of(&mut self, from: &str, to: &str, amount: u64) {
        if !self.can_write(from) {
            fail(format_args!("no write access to {from}"));
        }
        if !is_valid_address(to) {
            fail(format_args!("invalid recipient address {to}"));
        }
        let balance = self.account(from).balance;
        if balance < amount {
            fail(format_args!("insufficient balance of {from}: {balance} < {amount}"));
        }
        self.account_mut(from).balance -= amount;
        self.account_mut(to).balance += amount;
    }

    pub(crate) fn balance(&mut self) -> u64 {
        let callee = self.callee();
        self.balance_of(&callee)
    }

    pub(crate) fn balance_of(&mut self, address: &str) -> u64 {
        self.ledger.get(address).map(|a| a.balance).unwrap_or(0)
    }

    pub(crate) fn blake3(&mut self, data: &[u8]) -> [u8; 32] {
        *blake3::hash(data).as_bytes()
    }

    pub(crate) fn sha256(&mut self, data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(data).into()
    }

    pub(crate) fn keccak256(&mut self, data: &[u8]) -> [u8; 32] {
        use sha3::Digest;
        sha3::Keccak256::digest(data).into()
    }

    pub(crate) fn mimc(&mut self, data: &[u8]) -> [u8; 32] {
        match mimc::hash(data) {
            Ok(h) => h,
            Err(e) => fail(e),
        }
    }

    pub(crate) fn is_signature_valid(&mut self, _data: &str, _signature: &str, _public_key: &str) -> bool {
        self.signature_valid
    }

    pub(crate) fn is_evm_signature_valid(
        &mut self,
        _data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> bool {
        if signature.len() != 65 {
            fail("invalid EVM signature length, expected 65 bytes");
        }
        if public_key.len() != 64 {
            fail("invalid EVM public key length, expected 64 bytes");
        }
        self.evm_signature_valid
    }

    pub(crate) fn evm_get_address_from_pubkey(&mut self, public_key: &[u8]) -> Vec<u8> {
        if public_key.len() != 64 {
            fail("invalid EVM public key length, expected 64 bytes");
        }
        self.keccak256(public_key)[12..].to_vec()
    }

    pub(crate) fn evm_get_pubkey_from_signature(&mut self, _hash: &[u8], signature: &[u8]) -> Vec<u8> {
        if signature.len() != 65 {
            fail("invalid EVM signature length, expected 65 bytes");
        }
        self.evm_recovered_pubkey.clone()
    }

    pub(crate) fn is_address_eoa(&mut self, address: &str) -> bool {
        is_valid_address(address) && address.starts_with("AU")
    }

    pub(crate) fn public_key_to_address(&mut self, _public_key: &str) -> String {
        DEFAULT_CALLER.to_string()
    }

    pub(crate) fn validate_address(&mut self, address: &str) -> bool {
        is_valid_address(address)
    }

    pub(crate) fn get_op_keys(&mut self, prefix: &[u8]) -> Vec<Vec<u8>> {
        self.op_datastore.keys().filter(|k| k.starts_with(prefix)).cloned().collect()
    }

    pub(crate) fn has_op_key(&mut self, key: &[u8]) -> bool {
        self.op_datastore.contains_key(key)
    }

    pub(crate) fn get_op_data(&mut self, key: &[u8]) -> Vec<u8> {
        match self.op_datastore.get(key) {
            Some(v) => v.clone(),
            None => fail(format_args!("operation datastore key {key:?} not found")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_message(
        &mut self,
        address: &str,
        function: &str,
        validity_start: (u64, u8),
        validity_end: (u64, u8),
        max_gas: u64,
        fee: u64,
        coins: u64,
        args: &[u8],
        filter_address: &str,
        filter_key: &[u8],
    ) {
        let from = self.callee();
        let total = fee.checked_add(coins).unwrap_or_else(|| fail("fee + coins overflow"));
        let balance = self.account(&from).balance;
        if balance < total {
            fail(format_args!("insufficient balance to send message: {balance} < {total}"));
        }
        self.account_mut(&from).balance -= total;
        self.async_messages.push(AsyncMessage {
            address: address.to_string(),
            function: function.to_string(),
            validity_start,
            validity_end,
            max_gas,
            fee,
            coins,
            args: args.to_vec(),
            filter_address: filter_address.to_string(),
            filter_key: filter_key.to_vec(),
        });
    }

    pub(crate) fn deferred_call_quote(&mut self, _slot: (u64, u8), max_gas: u64, params_size: u64) -> u64 {
        1_000_000 + max_gas / 10 + params_size * 1_000
    }

    pub(crate) fn deferred_call_register(
        &mut self,
        address: &str,
        function: &str,
        slot: (u64, u8),
        max_gas: u64,
        args: &[u8],
        coins: u64,
    ) -> String {
        let price = self.deferred_call_quote(slot, max_gas, args.len() as u64);
        let from = self.callee();
        let total = price + coins;
        let balance = self.account(&from).balance;
        if balance < total {
            fail(format_args!("insufficient balance to register deferred call: {balance} < {total}"));
        }
        self.account_mut(&from).balance -= total;
        let id = std::format!("D12mock{}", self.deferred_calls.len() + 1);
        self.deferred_calls.push(DeferredCall {
            id: id.clone(),
            address: address.to_string(),
            function: function.to_string(),
            slot,
            max_gas,
            args: args.to_vec(),
            coins,
            scheduled: true,
        });
        id
    }

    pub(crate) fn deferred_call_exists(&mut self, id: &str) -> bool {
        self.deferred_calls.iter().any(|c| c.id == id && c.scheduled)
    }

    pub(crate) fn deferred_call_cancel(&mut self, id: &str) {
        let callee = self.callee();
        match self.deferred_calls.iter_mut().find(|c| c.id == id && c.scheduled) {
            Some(c) => {
                c.scheduled = false;
                let coins = c.coins;
                self.account_mut(&callee).balance += coins;
            }
            None => fail(format_args!("deferred call {id} not found")),
        }
    }
}

// ---------------------------------------------------------------------------
// test-facing helpers
// ---------------------------------------------------------------------------

/// Reset the VM to its default state (two addresses funded with
/// [`DEFAULT_BALANCE`], stack `[DEFAULT_CALLER, DEFAULT_CALLEE]`, no events).
pub fn reset() {
    VM.with(|vm| *vm.borrow_mut() = MockVm::default());
}

/// Replace the call stack (originator first, executing contract last).
pub fn set_call_stack(stack: &[&str]) {
    with_vm(|vm| {
        vm.call_stack = stack.iter().map(|s| s.to_string()).collect();
        for a in stack {
            vm.ledger.entry(a.to_string()).or_default();
        }
    })
}

/// Set the direct caller (the address just below the callee in the stack).
pub fn set_caller(caller: &Address) {
    with_vm(|vm| {
        let callee = vm.callee();
        vm.call_stack = std::vec![caller.to_string(), callee];
        vm.ledger.entry(caller.to_string()).or_default();
    })
}

/// Set the executing contract address (top of the stack).
pub fn set_callee(callee: &Address) {
    with_vm(|vm| {
        let caller = vm.caller();
        vm.call_stack = std::vec![caller, callee.to_string()];
        vm.ledger.entry(callee.to_string()).or_default();
    })
}

/// Simulate the deployment context: the caller has write access to the
/// contract (so [`crate::context::is_deploying_contract`] is `true`).
/// `caller` replaces the current caller when given.
pub fn set_deploy_context(caller: Option<&Address>) {
    with_vm(|vm| vm.admin_context = true);
    if let Some(c) = caller {
        set_caller(c);
    }
}

/// Simulate an `ExecuteSC`-like local context where caller == callee == `address`.
pub fn set_local_context(address: Option<&Address>) {
    with_vm(|vm| {
        let a = address.map(|a| a.to_string()).unwrap_or_else(|| vm.callee());
        vm.ledger.entry(a.clone()).or_default();
        vm.call_stack = std::vec![a];
    })
}

/// Force the result of [`crate::contract::caller_has_write_access`].
pub fn set_admin_context(admin: bool) {
    with_vm(|vm| vm.admin_context = admin)
}

/// Coins sent along with the current call ([`crate::context::transferred_coins`]).
pub fn set_transferred_coins(coins: u64) {
    with_vm(|vm| vm.transferred_coins = coins)
}

/// Set the balance of `address` (created when missing).
pub fn set_balance(address: &Address, balance: u64) {
    with_vm(|vm| vm.account_mut(address.as_str()).balance = balance)
}

/// Balance of `address`.
pub fn balance_of(address: &Address) -> u64 {
    with_vm(|vm| vm.balance_of(address.as_str()))
}

/// Slot timestamp in milliseconds.
pub fn set_time(time_ms: u64) {
    with_vm(|vm| vm.time = time_ms)
}

/// Current slot.
pub fn set_slot(period: u64, thread: u8) {
    with_vm(|vm| {
        vm.period = period;
        vm.thread = thread;
    })
}

/// Chain id reported by [`crate::context::chain_id`].
pub fn set_chain_id(chain_id: u64) {
    with_vm(|vm| vm.chain_id = chain_id)
}

/// Origin operation id (`None` for executions not started by an operation).
pub fn set_origin_operation_id(id: Option<&str>) {
    with_vm(|vm| vm.origin_operation_id = id.map(String::from))
}

/// Value reported by [`crate::context::remaining_gas`].
pub fn set_remaining_gas(gas: u64) {
    with_vm(|vm| vm.remaining_gas = gas)
}

/// Value reported by [`crate::crypto::unsafe_random`].
pub fn set_unsafe_random(value: i64) {
    with_vm(|vm| vm.random = value)
}

/// Result of [`crate::crypto::is_signature_valid`].
pub fn set_signature_valid(valid: bool) {
    with_vm(|vm| vm.signature_valid = valid)
}

/// Result of [`crate::crypto::is_evm_signature_valid`].
pub fn set_evm_signature_valid(valid: bool) {
    with_vm(|vm| vm.evm_signature_valid = valid)
}

/// Public key returned by [`crate::crypto::evm_get_pubkey_from_signature`].
pub fn set_evm_recovered_pubkey(pubkey: Vec<u8>) {
    with_vm(|vm| vm.evm_recovered_pubkey = pubkey)
}

/// Result of [`crate::contract::function_exists`].
pub fn set_function_exists(exists: bool) {
    with_vm(|vm| vm.function_exists = exists)
}

/// Set an operation datastore entry.
pub fn set_op_datastore_entry(key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
    with_vm(|vm| {
        vm.op_datastore.insert(key.into(), value.into());
    })
}

/// Set the bytecode of `address` (creating the address when needed).
pub fn set_bytecode_of(address: &Address, bytecode: impl Into<Vec<u8>>) {
    with_vm(|vm| vm.account_mut(address.as_str()).bytecode = bytecode.into())
}

/// Write directly into the datastore of any address (no access check).
pub fn set_storage_of(address: &Address, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
    with_vm(|vm| {
        vm.account_mut(address.as_str()).datastore.insert(key.into(), value.into());
    })
}

/// Snapshot of the datastore of `address`.
pub fn storage_of(address: &Address) -> BTreeMap<Vec<u8>, Vec<u8>> {
    with_vm(|vm| vm.ledger.get(address.as_str()).map(|a| a.datastore.clone()).unwrap_or_default())
}

/// Queue the return value of the next call to `function` at `address`.
pub fn mock_call(address: &Address, function: &str, ret: impl Into<Vec<u8>>) {
    with_vm(|vm| {
        vm.mocked_calls.push_back(MockedCall {
            address: Some(address.to_string()),
            function: Some(function.to_string()),
            ret: ret.into(),
        })
    })
}

/// Queue the return value of the next call to any contract/function (FIFO,
/// like the AssemblyScript `mockScCall`).
pub fn mock_any_call(ret: impl Into<Vec<u8>>) {
    with_vm(|vm| vm.mocked_calls.push_back(MockedCall { address: None, function: None, ret: ret.into() }))
}

/// Install a handler answering calls to other contracts:
/// `|address, function, args, coins| Some(return_bytes)`. Returning `None`
/// falls back to the queued mocks.
pub fn set_call_handler(handler: impl FnMut(&str, &str, &[u8], u64) -> Option<Vec<u8>> + 'static) {
    with_vm(|vm| vm.call_handler = Some(std::boxed::Box::new(handler)))
}

/// Calls made to other contracts so far.
pub fn recorded_calls() -> Vec<RecordedCall> {
    with_vm(|vm| vm.recorded_calls.clone())
}

/// Events emitted so far.
pub fn events() -> Vec<String> {
    with_vm(|vm| vm.events.clone())
}

/// Forget the events emitted so far.
pub fn clear_events() {
    with_vm(|vm| vm.events.clear())
}

/// Messages sent with [`crate::contract::async_call`].
pub fn async_messages() -> Vec<AsyncMessage> {
    with_vm(|vm| vm.async_messages.clone())
}

/// Deferred calls registered so far.
pub fn deferred_calls() -> Vec<DeferredCall> {
    with_vm(|vm| vm.deferred_calls.clone())
}

/// A fresh, deterministic, syntactically valid user address.
pub fn new_user_address() -> Address {
    Address::from(with_vm(|vm| {
        let a = vm.generated_address("AU");
        vm.ledger.entry(a.clone()).or_default();
        a
    }))
}

/// A fresh, deterministic, syntactically valid contract address.
pub fn new_sc_address() -> Address {
    Address::from(with_vm(|vm| {
        let a = vm.generated_address("AS");
        vm.ledger.entry(a.clone()).or_default();
        a
    }))
}

/// MiMC over the BN254 scalar field, matching the C-Layer node
/// (`gnark-crypto` compatible: 110 rounds, exponent 5, seed `"seed"`,
/// Miyaguchi-Preneel).
pub mod mimc {
    use ethnum::U256;
    use sha3::Digest;
    use std::string::String;
    use std::vec::Vec;

    // BN254 scalar field modulus r.
    const R: U256 = U256::from_words(0x30644e72e131a029b85045b68181585d, 0x2833e84879b9709143e1f593f0000001);
    const ROUNDS: usize = 110;

    fn add(a: U256, b: U256) -> U256 {
        let s = a + b; // < 2 * R < 2^255: no overflow
        if s >= R {
            s - R
        } else {
            s
        }
    }

    fn mul(a: U256, b: U256) -> U256 {
        let mut acc = U256::ZERO;
        for i in (0..256).rev() {
            acc = add(acc, acc);
            if (b >> i) & U256::ONE == U256::ONE {
                acc = add(acc, a);
            }
        }
        acc
    }

    fn pow5(x: U256) -> U256 {
        let x2 = mul(x, x);
        let x4 = mul(x2, x2);
        mul(x4, x)
    }

    fn from_be_mod(bytes: &[u8; 32]) -> U256 {
        U256::from_be_bytes(*bytes) % R
    }

    /// MiMC hash of `bytes` (same rules and result as the C-Layer node).
    pub fn hash(bytes: &[u8]) -> Result<[u8; 32], String> {
        if bytes.len() % 32 != 0 && bytes.len() > 32 {
            return Err("Input length must less than 32 or a multiple of 32".into());
        }
        if bytes.len() > 64 * 32 {
            return Err("Input length must less than 2048".into());
        }
        // round constants
        let mut hasher = sha3::Keccak256::new();
        hasher.update(b"seed");
        let mut rnd = hasher.finalize_reset();
        hasher.update(rnd);
        let mut constants = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            rnd = hasher.finalize_reset();
            constants.push(from_be_mod(&rnd.into()));
            hasher.update(rnd);
        }
        // data elements
        let mut elements = Vec::new();
        if !bytes.is_empty() {
            if bytes.len() < 32 {
                let mut padded = [0u8; 32];
                padded[32 - bytes.len()..].copy_from_slice(bytes);
                elements.push(from_be_mod(&padded));
            } else {
                for chunk in bytes.chunks(32) {
                    let arr: [u8; 32] = chunk.try_into().unwrap();
                    let v = U256::from_be_bytes(arr);
                    if v >= R {
                        return Err("Cannot convert into field element".into());
                    }
                    elements.push(v);
                }
            }
        }
        let mut h = U256::ZERO;
        for e in elements {
            let mut m = e;
            for c in &constants {
                m = pow5(add(add(m, h), *c));
            }
            m = add(m, h);
            h = add(add(m, h), e);
        }
        Ok(h.to_be_bytes())
    }

    #[cfg(test)]
    mod tests {
        use super::hash;

        #[test]
        fn matches_clayer_node_vectors() {
            assert_eq!(hash(b"").unwrap(), [0u8; 32]);
            let h1 = hash(b"Hello World").unwrap();
            assert_eq!(
                h1,
                [
                    37, 9, 120, 104, 178, 123, 147, 205, 1, 53, 154, 155, 95, 126, 42, 115, 62, 136, 182, 12,
                    227, 45, 22, 153, 180, 233, 123, 101, 206, 135, 162, 184
                ]
            );
            assert_eq!(
                hash(&h1).unwrap(),
                [
                    32, 230, 249, 72, 11, 8, 251, 253, 161, 246, 128, 243, 21, 63, 102, 187, 2, 212, 194,
                    126, 100, 195, 9, 100, 15, 135, 217, 110, 106, 38, 120, 245
                ]
            );
            assert_eq!(
                hash(&[h1, h1].concat()).unwrap(),
                [
                    31, 13, 212, 12, 150, 157, 144, 161, 249, 21, 94, 81, 53, 123, 172, 234, 151, 64, 249,
                    134, 149, 179, 241, 241, 49, 37, 128, 22, 128, 233, 208, 51
                ]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{coins, context, storage};

    #[test]
    fn defaults_and_isolation() {
        reset();
        assert_eq!(context::caller().as_str(), DEFAULT_CALLER);
        assert_eq!(context::callee().as_str(), DEFAULT_CALLEE);
        assert_eq!(coins::balance(), DEFAULT_BALANCE);
        storage::set("a", "b");
        reset();
        assert!(!storage::has("a"));
    }

    #[test]
    #[should_panic(expected = "no write access")]
    fn foreign_storage_is_protected() {
        reset();
        storage::set_of(&Address::from(DEFAULT_CALLER), "k", "v");
    }

    #[test]
    fn generated_addresses_are_valid_and_unique() {
        reset();
        let a = new_user_address();
        let b = new_sc_address();
        assert_ne!(a.as_str(), b.as_str());
        assert!(a.is_eoa());
        assert!(b.is_smart_contract());
        assert!(crate::crypto::validate_address(a.as_str()));
    }
}
