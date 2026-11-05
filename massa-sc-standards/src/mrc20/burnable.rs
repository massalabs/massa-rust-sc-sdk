//! Burnable extension: holders (or approved spenders) can destroy tokens.

use massa_sc_sdk::prelude::*;

use super::internals::{allowance, approve, balance, set_balance, set_total_supply, total_supply};

/// Event emitted by burns.
pub const BURN_EVENT: &str = "BURN_SUCCESS";

/// Destroy `amount` tokens of `owner` (no access check, no event).
pub fn burn(owner: &Address, amount: U256) {
    decrease_total_supply(amount);
    decrease_balance(owner, amount);
}

/// Remove `amount` from the balance of `owner`; aborts on underflow.
pub fn decrease_balance(owner: &Address, amount: U256) {
    let old = balance(owner);
    let new =
        old.checked_sub(amount).expect("Requested burn amount causes an underflow of the recipient balance");
    set_balance(owner, new);
}

/// Remove `amount` from the total supply; aborts on underflow.
pub fn decrease_total_supply(amount: U256) {
    let old = total_supply();
    let new = old.checked_sub(amount).expect("Requested burn amount causes an underflow of the total supply");
    set_total_supply(new);
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `burn(amount: u256)`: burn the caller's tokens.
    pub fn burn(mut args: Args) {
        let initial = coins::balance();
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        super::burn(&context::caller(), amount);
        generate_event(BURN_EVENT);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `burnFrom(owner: string, amount: u256)`: burn using the caller's allowance.
    pub fn burn_from(mut args: Args) {
        let initial = coins::balance();
        let owner: Address = args.must_next("owner argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        let spender = context::caller();
        let spender_allowance = allowance(&owner, &spender);
        assert!(spender_allowance >= amount, "burnFrom failed: insufficient allowance");
        super::burn(&owner, amount);
        approve(&owner, &spender, spender_allowance - amount);
        generate_event(BURN_EVENT);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export `burn(amount)` and `burnFrom(owner, amount)`.
#[macro_export]
macro_rules! export_mrc20_burnable {
    () => {
        #[::massa_sc_sdk::export(name = "burn")]
        pub fn __std_burn(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::burnable::entrypoints::burn(args)
        }
        #[::massa_sc_sdk::export(name = "burnFrom")]
        pub fn __std_burn_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::burnable::entrypoints::burn_from(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn burn_flows() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc20::constructor("T", "T", 9, U256::from(100u64));
        mock::set_admin_context(false);
        let owner = context::caller();
        entrypoints::burn(Args::new().add(U256::from(30u64)));
        assert_eq!(balance(&owner), U256::from(70u64));
        assert_eq!(total_supply(), U256::from(70u64));

        let bob = mock::new_user_address();
        approve(&owner, &bob, U256::from(20u64));
        mock::set_caller(&bob);
        entrypoints::burn_from(Args::new().add(&owner).add(U256::from(20u64)));
        assert_eq!(balance(&owner), U256::from(50u64));
        assert_eq!(allowance(&owner, &bob), U256::ZERO);
    }

    #[test]
    #[should_panic(expected = "insufficient allowance")]
    fn burn_from_needs_allowance() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc20::constructor("T", "T", 9, U256::from(100u64));
        mock::set_admin_context(false);
        let owner = context::caller();
        mock::set_caller(&mock::new_user_address());
        entrypoints::burn_from(Args::new().add(&owner).add(U256::ONE));
    }
}
