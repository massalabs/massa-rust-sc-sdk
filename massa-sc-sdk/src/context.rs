//! Execution context: who is calling, which contract is running, coins sent
//! with the call, time, slot, gas, chain.
//!
//! The call stack semantics (bottom ... top):
//! * `ExecuteSC` by account A: `[A]`
//! * `CallSC` by A on contract B: `[A, B]`
//! * B's function calling contract C: `[A, B, C]` while C runs
//! * asynchronous message / deferred call sent while the stack was
//!   `[A, B, C]` and targeting D: `[C, D]`
//! * local executions do not change the stack.

use alloc::string::String;
use alloc::vec::Vec;

use crate::args::{Args, ArgsDecode, ArgsEncode, Result as ArgsResult};
use crate::{env, Address};

/// A block slot: `(period, thread)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Slot {
    /// Period number.
    pub period: u64,
    /// Thread index (`0..THREAD_COUNT`).
    pub thread: u8,
}

impl Slot {
    /// A slot from its coordinates.
    pub const fn new(period: u64, thread: u8) -> Self {
        Self { period, thread }
    }

    /// Slot of the current execution.
    pub fn current() -> Self {
        Self { period: env::current_period(), thread: env::current_thread() }
    }

    /// The slot `n` slots after this one (threads wrap around to the next period).
    pub fn plus(self, n: u64) -> Self {
        let threads = crate::constants::THREAD_COUNT as u64;
        let total = self.thread as u64 + n;
        Self { period: self.period + total / threads, thread: (total % threads) as u8 }
    }
}

impl From<(u64, u8)> for Slot {
    fn from((period, thread): (u64, u8)) -> Self {
        Self { period, thread }
    }
}

impl From<Slot> for (u64, u8) {
    fn from(s: Slot) -> Self {
        (s.period, s.thread)
    }
}

impl ArgsEncode for Slot {
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.period.encode_into(out);
        self.thread.encode_into(out);
    }
}

impl ArgsDecode for Slot {
    fn decode(args: &mut Args) -> ArgsResult<Self> {
        Ok(Self { period: args.next()?, thread: args.next()? })
    }
}

/// Addresses in the current call stack, originator first, current contract last.
pub fn call_stack() -> Vec<Address> {
    env::call_stack().into_iter().map(Address::from).collect()
}

/// Address of the currently executing contract (top of the stack).
pub fn callee() -> Address {
    match env::call_stack_from_top(0) {
        Some(a) => Address::from(a),
        None => crate::abort("empty call stack"),
    }
}

/// Address that directly called the current function: the account or contract
/// just below the callee in the stack. When there is none (`ExecuteSC`), the
/// callee itself.
pub fn caller() -> Address {
    match env::call_stack_from_top(1) {
        Some(a) => Address::from(a),
        None => callee(),
    }
}

/// Account that created the operation at the origin of this execution
/// (bottom of the stack).
pub fn transaction_creator() -> Address {
    match env::call_stack().into_iter().next() {
        Some(a) => Address::from(a),
        None => crate::abort("empty call stack"),
    }
}

/// Addresses the current execution may write to: the current address plus the
/// addresses it created during this execution.
pub fn owned_addresses() -> Vec<Address> {
    env::owned_addresses().into_iter().map(Address::from).collect()
}

/// Coins (nanoMAS) sent along with the current call. Unrelated to
/// [`crate::coins::transfer_coins`].
#[inline]
pub fn transferred_coins() -> u64 {
    env::call_coins()
}

/// Slot timestamp in milliseconds since the Unix epoch.
#[inline]
pub fn timestamp() -> u64 {
    env::time()
}

/// Gas still available to the current execution.
#[inline]
pub fn remaining_gas() -> u64 {
    env::remaining_gas()
}

/// Period of the current slot.
#[inline]
pub fn current_period() -> u64 {
    env::current_period()
}

/// Thread of the current slot.
#[inline]
pub fn current_thread() -> u8 {
    env::current_thread()
}

/// The current slot.
#[inline]
pub fn current_slot() -> Slot {
    Slot::current()
}

/// Chain id of the network (see [`crate::constants`]).
#[inline]
pub fn chain_id() -> u64 {
    env::chain_id()
}

/// `true` when the current function runs during the deployment of this
/// contract: the caller has write access to the contract (it just created it)
/// and is not the contract itself. Use it to protect `constructor`.
pub fn is_deploying_contract() -> bool {
    env::caller_has_write_access() && callee() != caller()
}

/// Id of the operation that originated the execution, `None` for executions
/// not started by an operation (asynchronous messages, deferred calls).
pub fn origin_operation_id() -> Option<String> {
    let id = env::origin_operation_id();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// `true` on any C-Layer network (where the C-Layer specific ABIs exist).
pub fn is_clayer() -> bool {
    use crate::constants::*;
    matches!(env::chain_id(), CLAYER_MAINNET_CHAIN_ID | CLAYER_TESTNET_CHAIN_ID | CLAYER_BUILDNET_CHAIN_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn stack_helpers() {
        mock::reset();
        let a = Address::from("AU1a");
        let b = Address::from("AS1b");
        let c = Address::from("AS1c");
        mock::set_call_stack(&[a.as_str(), b.as_str(), c.as_str()]);
        assert_eq!(callee(), c);
        assert_eq!(caller(), b);
        assert_eq!(transaction_creator(), a);
        assert_eq!(call_stack(), vec![a.clone(), b, c]);
        mock::set_call_stack(&[a.as_str()]);
        assert_eq!(caller(), a);
        assert_eq!(callee(), a);
    }

    #[test]
    fn slot_arithmetic() {
        assert_eq!(Slot::new(10, 30).plus(5), Slot::new(11, 3));
        assert_eq!(Slot::new(10, 0).plus(64), Slot::new(12, 0));
    }

    #[test]
    fn deploying_context() {
        mock::reset();
        assert!(!is_deploying_contract());
        mock::set_deploy_context(None);
        assert!(is_deploying_contract());
    }
}
