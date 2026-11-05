//! MRC20 fungible token (Massa's ERC20), storage-compatible with
//! `@massalabs/sc-standards` `MRC20`.
//!
//! * [`internals`]: typed, storage-level primitives to build on
//!   (`balance`, `transfer`, `approve`, ...). They never reconcile coins.
//! * [`entrypoints`]: the standard functions taking serialized arguments and
//!   reconciling storage costs with [`coins::transfer_remaining`].
//! * [`crate::export_mrc20!`], [`crate::export_mrc20_mintable!`],
//!   [`crate::export_mrc20_burnable!`]: expose them from a contract.
//! * [`Mrc20`]: client to call a token deployed at some address.
//!
//! ```ignore
//! use massa_sc_sdk::prelude::*;
//! use massa_sc_standards::{export_mrc20, export_ownable, mrc20};
//!
//! #[export]
//! pub fn constructor(mut args: Args) {
//!     let name: String = args.must_next("name");
//!     let symbol: String = args.must_next("symbol");
//!     let decimals: u8 = args.must_next("decimals");
//!     let supply: U256 = args.must_next("totalSupply");
//!     mrc20::constructor(&name, &symbol, decimals, supply);
//! }
//! export_mrc20!();
//! export_ownable!();
//! ```

pub mod burnable;
pub mod external;
pub mod internals;
pub mod mintable;
pub mod wrapper;

pub use wrapper::Mrc20;

use massa_sc_sdk::prelude::*;

use crate::ownership;

pub const VERSION: &str = "0.1.0";
pub const NAME_KEY: &str = "NAME";
pub const SYMBOL_KEY: &str = "SYMBOL";
pub const TOTAL_SUPPLY_KEY: &str = "TOTAL_SUPPLY";
pub const DECIMALS_KEY: &str = "DECIMALS";
pub const TRANSFER_EVENT_NAME: &str = "TRANSFER SUCCESS";
pub const APPROVAL_EVENT_NAME: &str = "APPROVAL SUCCESS";

/// Initialize the token: metadata, total supply credited to the caller, and
/// the caller as owner. Call it from your `constructor`; aborts when not
/// called during deployment.
pub fn constructor(name: &str, symbol: &str, decimals: u8, total_supply: U256) {
    assert!(context::is_deploying_contract(), "constructor must be called at deployment");
    storage::set(NAME_KEY, name);
    storage::set(SYMBOL_KEY, symbol);
    storage::set(DECIMALS_KEY, [decimals]);
    storage::set(TOTAL_SUPPLY_KEY, total_supply.to_le_bytes());
    let caller = context::caller();
    ownership::set_owner(&caller);
    internals::set_balance(&caller, total_supply);
}

pub fn name() -> String {
    String::from_utf8_lossy(&storage::get(NAME_KEY)).into_owned()
}

pub fn symbol() -> String {
    String::from_utf8_lossy(&storage::get(SYMBOL_KEY)).into_owned()
}

pub fn decimals() -> u8 {
    storage::get(DECIMALS_KEY).first().copied().unwrap_or(0)
}

pub fn total_supply() -> U256 {
    internals::read_u256(&storage::get(TOTAL_SUPPLY_KEY))
}

/// Entry points taking serialized arguments (the on-chain interface).
pub mod entrypoints {
    use super::*;
    use internals::*;

    /// `version() -> string bytes`
    pub fn version() -> Vec<u8> {
        VERSION.as_bytes().to_vec()
    }

    /// `name() -> string bytes`
    pub fn name() -> Vec<u8> {
        storage::get(NAME_KEY)
    }

    /// `symbol() -> string bytes`
    pub fn symbol() -> Vec<u8> {
        storage::get(SYMBOL_KEY)
    }

    /// `decimals() -> u8`
    pub fn decimals() -> Vec<u8> {
        storage::get(DECIMALS_KEY)
    }

    /// `totalSupply() -> u256`
    pub fn total_supply() -> Vec<u8> {
        storage::get(TOTAL_SUPPLY_KEY)
    }

    /// `balanceOf(owner: string) -> u256`
    pub fn balance_of(mut args: Args) -> U256 {
        let owner: Address = args.must_next("Address argument is missing or invalid");
        balance(&owner)
    }

    /// `transfer(to: string, amount: u256)`
    ///
    /// The caller pays the storage of a new recipient balance entry: send
    /// enough coins with the call (see [`external::balance_entry_cost`]);
    /// the surplus is refunded.
    pub fn transfer(mut args: Args) {
        let initial = coins::balance();
        let owner = context::caller();
        let to: Address = args.must_next("receiverAddress argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        internals::transfer(&owner, &to, amount);
        generate_event(TRANSFER_EVENT_NAME);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `allowance(owner: string, spender: string) -> u256`
    pub fn allowance(mut args: Args) -> U256 {
        let owner: Address = args.must_next("owner argument is missing or invalid");
        let spender: Address = args.must_next("spenderAddress argument is missing or invalid");
        internals::allowance(&owner, &spender)
    }

    /// `increaseAllowance(spender: string, amount: u256)` (saturating at `U256::MAX`).
    pub fn increase_allowance(mut args: Args) {
        let initial = coins::balance();
        let owner = context::caller();
        let spender: Address = args.must_next("spenderAddress argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        let new = internals::allowance(&owner, &spender).saturating_add(amount);
        approve(&owner, &spender, new);
        generate_event(APPROVAL_EVENT_NAME);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `decreaseAllowance(spender: string, amount: u256)` (saturating at 0).
    pub fn decrease_allowance(mut args: Args) {
        let initial = coins::balance();
        let owner = context::caller();
        let spender: Address = args.must_next("spenderAddress argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        let new = internals::allowance(&owner, &spender).saturating_sub(amount);
        approve(&owner, &spender, new);
        generate_event(APPROVAL_EVENT_NAME);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `transferFrom(owner: string, recipient: string, amount: u256)`
    pub fn transfer_from(mut args: Args) {
        let initial = coins::balance();
        let spender = context::caller();
        let owner: Address = args.must_next("ownerAddress argument is missing or invalid");
        let recipient: Address = args.must_next("recipientAddress argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        let spender_allowance = internals::allowance(&owner, &spender);
        assert!(spender_allowance >= amount, "transferFrom failed: insufficient allowance");
        internals::transfer(&owner, &recipient, amount);
        approve(&owner, &spender, spender_allowance - amount);
        generate_event(TRANSFER_EVENT_NAME);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export the MRC20 entry points: `version`, `name`, `symbol`, `decimals`,
/// `totalSupply`, `balanceOf`, `transfer`, `allowance`, `increaseAllowance`,
/// `decreaseAllowance`, `transferFrom`. You still define `constructor`
/// (calling [`constructor`]) and usually add [`crate::export_ownable!`].
#[macro_export]
macro_rules! export_mrc20 {
    () => {
        #[::massa_sc_sdk::export(name = "version")]
        pub fn __std_version() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc20::entrypoints::version()
        }
        #[::massa_sc_sdk::export(name = "name")]
        pub fn __std_name() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc20::entrypoints::name()
        }
        #[::massa_sc_sdk::export(name = "symbol")]
        pub fn __std_symbol() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc20::entrypoints::symbol()
        }
        #[::massa_sc_sdk::export(name = "decimals")]
        pub fn __std_decimals() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc20::entrypoints::decimals()
        }
        #[::massa_sc_sdk::export(name = "totalSupply")]
        pub fn __std_total_supply() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc20::entrypoints::total_supply()
        }
        #[::massa_sc_sdk::export(name = "balanceOf")]
        pub fn __std_balance_of(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::U256 {
            $crate::mrc20::entrypoints::balance_of(args)
        }
        #[::massa_sc_sdk::export(name = "transfer")]
        pub fn __std_transfer(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::entrypoints::transfer(args)
        }
        #[::massa_sc_sdk::export(name = "allowance")]
        pub fn __std_allowance(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::U256 {
            $crate::mrc20::entrypoints::allowance(args)
        }
        #[::massa_sc_sdk::export(name = "increaseAllowance")]
        pub fn __std_increase_allowance(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::entrypoints::increase_allowance(args)
        }
        #[::massa_sc_sdk::export(name = "decreaseAllowance")]
        pub fn __std_decrease_allowance(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::entrypoints::decrease_allowance(args)
        }
        #[::massa_sc_sdk::export(name = "transferFrom")]
        pub fn __std_transfer_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::entrypoints::transfer_from(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    fn deploy() -> (Address, Address) {
        mock::reset();
        mock::set_deploy_context(None);
        constructor("MassaToken", "MT", 18, U256::from(1_000_000u64));
        mock::set_admin_context(false);
        mock::clear_events();
        (context::caller(), context::callee())
    }

    #[test]
    fn metadata_and_supply() {
        let (owner, _) = deploy();
        assert_eq!(name(), "MassaToken");
        assert_eq!(symbol(), "MT");
        assert_eq!(decimals(), 18);
        assert_eq!(total_supply(), U256::from(1_000_000u64));
        assert_eq!(internals::balance(&owner), U256::from(1_000_000u64));
        assert_eq!(entrypoints::version(), b"0.1.0");
        assert_eq!(entrypoints::balance_of(Args::new().add(&owner)), U256::from(1_000_000u64));
        assert_eq!(
            storage::get("BALANCE".to_string() + owner.as_str()),
            U256::from(1_000_000u64).to_le_bytes()
        );
        assert!(ownership::is_owner(&owner));
    }

    #[test]
    fn transfer_and_allowances() {
        let (owner, _) = deploy();
        let bob = mock::new_user_address();
        entrypoints::transfer(Args::new().add(&bob).add(U256::from(100u64)));
        assert_eq!(internals::balance(&bob), U256::from(100u64));
        assert_eq!(internals::balance(&owner), U256::from(999_900u64));
        assert_eq!(mock::events(), vec![TRANSFER_EVENT_NAME]);

        entrypoints::increase_allowance(Args::new().add(&bob).add(U256::from(50u64)));
        assert_eq!(internals::allowance(&owner, &bob), U256::from(50u64));
        entrypoints::decrease_allowance(Args::new().add(&bob).add(U256::from(80u64)));
        assert_eq!(internals::allowance(&owner, &bob), U256::ZERO);
        entrypoints::increase_allowance(Args::new().add(&bob).add(U256::MAX));
        entrypoints::increase_allowance(Args::new().add(&bob).add(U256::ONE));
        assert_eq!(internals::allowance(&owner, &bob), U256::MAX);

        // bob spends owner's allowance
        mock::set_caller(&bob);
        let carol = mock::new_user_address();
        entrypoints::transfer_from(Args::new().add(&owner).add(&carol).add(U256::from(10u64)));
        assert_eq!(internals::balance(&carol), U256::from(10u64));
        assert_eq!(internals::allowance(&owner, &bob), U256::MAX - U256::from(10u64));
    }

    #[test]
    #[should_panic(expected = "insufficient funds")]
    fn transfer_more_than_balance() {
        let (_, _) = deploy();
        let bob = mock::new_user_address();
        mock::set_caller(&bob);
        entrypoints::transfer(Args::new().add(mock::new_user_address()).add(U256::ONE));
    }

    #[test]
    fn zero_balance_entries_are_deleted() {
        let (owner, _) = deploy();
        let bob = mock::new_user_address();
        entrypoints::transfer(Args::new().add(&bob).add(U256::from(1_000_000u64)));
        assert!(!storage::has(internals::balance_key(&owner)));
        assert_eq!(internals::balance(&owner), U256::ZERO);
        // self transfer is a no-op
        mock::set_caller(&bob);
        entrypoints::transfer(Args::new().add(&bob).add(U256::from(5u64)));
        assert_eq!(internals::balance(&bob), U256::from(1_000_000u64));
    }
}
