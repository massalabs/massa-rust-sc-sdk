//! MRC1155 burnable extension: holders (or their operators) can destroy tokens.

use massa_sc_sdk::prelude::*;

use super::internals::{self, is_approved_for_all, MISSING_APPROVAL_FOR_ALL_ERROR};

pub use internals::{burn, burn_batch};

pub mod entrypoints {
    use super::*;

    /// `burn(account: string, id: u256, value: u256)`
    pub fn burn(mut args: Args) {
        let initial = coins::balance();
        let sender = context::caller();
        let account: Address = args.must_next("account argument is missing or invalid");
        let id: U256 = args.must_next("id argument is missing or invalid");
        let value: U256 = args.must_next("value argument is missing or invalid");
        assert!(
            account == sender || is_approved_for_all(&account, &sender),
            "{MISSING_APPROVAL_FOR_ALL_ERROR}"
        );
        internals::burn(&account, id, value);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `burnBatch(account: string, ids: u256[], values: u256[])`
    pub fn burn_batch(mut args: Args) {
        let initial = coins::balance();
        let sender = context::caller();
        let account: Address = args.must_next("account argument is missing or invalid");
        let ids: Vec<U256> = args.must_next("ids argument is missing or invalid");
        let values: Vec<U256> = args.must_next("values argument is missing or invalid");
        assert!(
            account == sender || is_approved_for_all(&account, &sender),
            "{MISSING_APPROVAL_FOR_ALL_ERROR}"
        );
        internals::burn_batch(&account, &ids, &values);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export `burn` and `burnBatch`.
#[macro_export]
macro_rules! export_mrc1155_burnable {
    () => {
        #[::massa_sc_sdk::export(name = "burn")]
        pub fn __std_burn(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::burnable::entrypoints::burn(args)
        }
        #[::massa_sc_sdk::export(name = "burnBatch")]
        pub fn __std_burn_batch(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::burnable::entrypoints::burn_batch(args)
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
        crate::mrc1155::constructor("u");
        mock::set_admin_context(false);
        let owner = context::caller();
        internals::mint(&owner, U256::ONE, U256::from(10u64), &[]);
        entrypoints::burn(Args::new().add(&owner).add(U256::ONE).add(U256::from(4u64)));
        assert_eq!(internals::balance_of(&owner, U256::ONE), U256::from(6u64));
        entrypoints::burn_batch(Args::new().add(&owner).add(vec![U256::ONE]).add(vec![U256::from(6u64)]));
        assert_eq!(internals::balance_of(&owner, U256::ONE), U256::ZERO);
    }
}
