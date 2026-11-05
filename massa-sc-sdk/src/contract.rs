//! Interacting with other contracts: calls, deployment, bytecode,
//! asynchronous messages.

use alloc::vec::Vec;

use crate::context::Slot;
use crate::{env, Address};

/// Call `function` of the contract at `address` with serialized `args`,
/// sending `coins` nanoMAS along. Returns the raw bytes returned by the callee.
///
/// `args` is anything byte-like: `&Args`, `&[u8]`, `Vec<u8>`...
/// Aborts when the address or the function does not exist, or when the callee
/// aborts.
#[inline]
pub fn call(address: &Address, function: &str, args: impl AsRef<[u8]>, coins: u64) -> Vec<u8> {
    env::call(address.as_str(), function, args.as_ref(), coins)
}

/// Run `function` of the contract at `address` *in the current context*: the
/// code of the other contract executes as if it were part of this one
/// (reads and writes this contract's datastore, balance and call stack).
#[inline]
pub fn local_call(address: &Address, function: &str, args: impl AsRef<[u8]>) -> Vec<u8> {
    env::local_call(address.as_str(), function, args.as_ref())
}

/// Run `function` of the given `bytecode` in the current context.
#[inline]
pub fn local_execution(bytecode: &[u8], function: &str, args: impl AsRef<[u8]>) -> Vec<u8> {
    env::local_execution(bytecode, function, args.as_ref())
}

/// Deploy `bytecode` at a new address and return it. The current execution
/// gets write access to the new address until it ends (to initialize its
/// datastore, send it coins, or call its `constructor`).
///
/// The storage of the new address ([`deploy_cost`]) is paid by the current
/// contract: make sure its balance covers it (e.g. from the coins sent with
/// the call), or the execution aborts.
#[inline]
pub fn create_sc(bytecode: &[u8]) -> Address {
    Address::from(env::create_sc(bytecode))
}

/// Coins (nanoMAS) the creator pays when [`create_sc`] deploys `bytecode_len`
/// bytes on a Massa network: a new ledger entry plus one storage byte per
/// bytecode byte (a 20 KiB contract costs about 2 MAS). C-Layer networks
/// charge far less (see [`crate::constants::CLAYER_STORAGE_BYTE_COST`]).
pub const fn deploy_cost(bytecode_len: usize) -> u64 {
    crate::constants::NEW_LEDGER_ENTRY_COST + bytecode_len as u64 * crate::constants::STORAGE_BYTE_COST
}

/// `true` when the contract at `address` exports `function`.
#[inline]
pub fn function_exists(address: &Address, function: &str) -> bool {
    env::function_exists(address.as_str(), function)
}

/// `true` when the caller has write access to the current contract, which
/// only happens in the execution that created it (see [`create_sc`]).
#[inline]
pub fn caller_has_write_access() -> bool {
    env::caller_has_write_access()
}

/// Bytecode of the current contract.
#[inline]
pub fn get_bytecode() -> Vec<u8> {
    env::get_bytecode()
}

/// Bytecode of the contract at `address` (aborts when the address does not exist).
#[inline]
pub fn get_bytecode_of(address: &Address) -> Vec<u8> {
    env::get_bytecode_of(address.as_str())
}

/// Replace the bytecode of the current contract (max 10 MB).
#[inline]
pub fn set_bytecode(bytecode: &[u8]) {
    env::set_bytecode(bytecode)
}

/// Replace the bytecode of `address` (requires write access).
#[inline]
pub fn set_bytecode_of(address: &Address, bytecode: &[u8]) {
    env::set_bytecode_of(address.as_str(), bytecode)
}

/// Abort unless `address` is a valid, deployed and not destroyed smart contract.
pub fn assert_is_smart_contract(address: &Address) {
    assert!(address.is_valid(), "{address} is not a valid address");
    assert!(address.as_str().starts_with("AS"), "{address} is not a smart contract address");
    assert!(!get_bytecode_of(address).is_empty(), "No bytecode found at address: {address}");
}

/// Optional trigger of an asynchronous message: the message is executed only
/// once `address` (or its datastore `key`) was modified within the validity range.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageFilter {
    /// Address whose modification triggers the message (`None` = no filter).
    pub address: Option<Address>,
    /// Datastore key of `address` whose modification triggers the message (empty = any change of the address).
    pub key: Vec<u8>,
}

impl MessageFilter {
    /// Trigger on any change of `address`.
    pub fn address(address: Address) -> Self {
        Self { address: Some(address), key: Vec::new() }
    }
    /// Trigger on a change of `key` in the datastore of `address`.
    pub fn datastore_key(address: Address, key: impl Into<Vec<u8>>) -> Self {
        Self { address: Some(address), key: key.into() }
    }
}

/// Schedule the execution of `function` at `address`, as soon as possible
/// within `[validity_start, validity_end]` (legacy "autonomous smart
/// contract" message). Prefer [`crate::deferred_calls`] for a guaranteed slot.
///
/// * `max_gas`: gas allocated to the execution.
/// * `fee`: priority fee paid now by the current contract.
/// * `coins`: coins transferred to the target with the call.
/// * `filter`: optional trigger condition.
#[allow(clippy::too_many_arguments)]
pub fn async_call(
    address: &Address,
    function: &str,
    validity_start: Slot,
    validity_end: Slot,
    max_gas: u64,
    fee: u64,
    coins: u64,
    args: impl AsRef<[u8]>,
    filter: Option<MessageFilter>,
) {
    let filter = filter.unwrap_or_default();
    env::send_message(
        address.as_str(),
        function,
        validity_start.into(),
        validity_end.into(),
        max_gas,
        fee,
        coins,
        args.as_ref(),
        filter.address.as_ref().map(|a| a.as_str()).unwrap_or(""),
        &filter.key,
    )
}

/// Low-level form of [`async_call`] mirroring the host ABI.
#[allow(clippy::too_many_arguments)]
pub fn send_message(
    address: &Address,
    function: &str,
    validity_start_period: u64,
    validity_start_thread: u8,
    validity_end_period: u64,
    validity_end_thread: u8,
    max_gas: u64,
    raw_fee: u64,
    coins: u64,
    args: impl AsRef<[u8]>,
    filter_address: Option<&Address>,
    filter_key: &[u8],
) {
    env::send_message(
        address.as_str(),
        function,
        (validity_start_period, validity_start_thread),
        (validity_end_period, validity_end_thread),
        max_gas,
        raw_fee,
        coins,
        args.as_ref(),
        filter_address.map(|a| a.as_str()).unwrap_or(""),
        filter_key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mock, Args};

    #[test]
    fn mocked_calls() {
        mock::reset();
        let target = Address::from("AS1target");
        mock::mock_call(&target, "get_value", Args::new().add(7u64).into_bytes());
        let ret = call(&target, "get_value", Args::new(), 0);
        assert_eq!(Args::from(ret).next_u64().unwrap(), 7);
        let calls = mock::recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function, "get_value");
    }

    #[test]
    fn create_and_write_child() {
        mock::reset();
        let child = create_sc(b"\0asm");
        assert!(child.as_str().starts_with("AS"));
        crate::storage::set_of(&child, "k", "v");
        assert_eq!(crate::storage::get_of(&child, "k"), b"v");
        assert_eq!(get_bytecode_of(&child), b"\0asm");
        assert!(crate::context::owned_addresses().contains(&child));
    }
}
