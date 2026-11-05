//! Mintable extension: the owner can create new tokens.

use massa_sc_sdk::prelude::*;

use super::internals::{balance, set_balance, set_total_supply, total_supply};
use crate::ownership;

/// Event emitted by mints.
pub const MINT_EVENT: &str = "MINT SUCCESS";

/// Credit `amount` new tokens to `recipient` (no access check, no event).
pub fn mint(recipient: &Address, amount: U256) {
    increase_total_supply(amount);
    increase_balance(recipient, amount);
}

/// Add `amount` to the balance of `recipient`; aborts on overflow.
pub fn increase_balance(recipient: &Address, amount: U256) {
    let old = balance(recipient);
    let new = old.checked_add(amount).expect("Requested mint amount causes an overflow");
    set_balance(recipient, new);
}

/// Add `amount` to the total supply; aborts on overflow.
pub fn increase_total_supply(amount: U256) {
    let old = total_supply();
    let new = old.checked_add(amount).expect("Requested mint amount causes an overflow");
    set_total_supply(new);
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `mint(recipient: string, amount: u256)`, owner only.
    pub fn mint(mut args: Args) {
        let initial = coins::balance();
        ownership::only_owner();
        let recipient: Address = args.must_next("recipient argument is missing or invalid");
        let amount: U256 = args.must_next("amount argument is missing or invalid");
        super::mint(&recipient, amount);
        generate_event(MINT_EVENT);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export `mint(recipient, amount)` (owner only).
#[macro_export]
macro_rules! export_mrc20_mintable {
    () => {
        #[::massa_sc_sdk::export(name = "mint")]
        pub fn __std_mint(args: ::massa_sc_sdk::Args) {
            $crate::mrc20::mintable::entrypoints::mint(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn owner_mints() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc20::constructor("T", "T", 9, U256::from(10u64));
        mock::set_admin_context(false);
        let bob = mock::new_user_address();
        entrypoints::mint(Args::new().add(&bob).add(U256::from(5u64)));
        assert_eq!(balance(&bob), U256::from(5u64));
        assert_eq!(total_supply(), U256::from(15u64));
    }

    #[test]
    #[should_panic(expected = "Caller is not the owner")]
    fn others_cannot_mint() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc20::constructor("T", "T", 9, U256::from(10u64));
        mock::set_admin_context(false);
        mock::set_caller(&mock::new_user_address());
        entrypoints::mint(Args::new().add(context::caller()).add(U256::ONE));
    }
}
