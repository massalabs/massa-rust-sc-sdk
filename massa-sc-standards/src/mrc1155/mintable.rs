//! MRC1155 mintable extension: accounts holding the `minter` role
//! ([`crate::access_control`]) can mint.

use massa_sc_sdk::prelude::*;

use super::internals;
use crate::access_control::only_role;

pub const MINTER_ROLE: &str = "minter";

pub use internals::{mint, mint_batch};

pub mod entrypoints {
    use super::*;

    /// `mint(to: string, id: u256, value: u256, data: bytes)`, `minter` role only.
    pub fn mint(mut args: Args) {
        let initial = coins::balance();
        only_role(MINTER_ROLE);
        let to: Address = args.must_next("to argument is missing or invalid");
        let id: U256 = args.must_next("id argument is missing or invalid");
        let value: U256 = args.must_next("value argument is missing or invalid");
        let data: Vec<u8> = args.must_next("data argument is missing or invalid");
        internals::mint(&to, id, value, &data);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `mintBatch(to: string, ids: u256[], values: u256[], data: bytes)`, `minter` role only.
    pub fn mint_batch(mut args: Args) {
        let initial = coins::balance();
        only_role(MINTER_ROLE);
        let to: Address = args.must_next("to argument is missing or invalid");
        let ids: Vec<U256> = args.must_next("ids argument is missing or invalid");
        let values: Vec<U256> = args.must_next("values argument is missing or invalid");
        let data: Vec<u8> = args.must_next("data argument is missing or invalid");
        internals::mint_batch(&to, &ids, &values, &data);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export `mint` and `mintBatch` (`minter` role).
#[macro_export]
macro_rules! export_mrc1155_mintable {
    () => {
        #[::massa_sc_sdk::export(name = "mint")]
        pub fn __std_mint(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::mintable::entrypoints::mint(args)
        }
        #[::massa_sc_sdk::export(name = "mintBatch")]
        pub fn __std_mint_batch(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::mintable::entrypoints::mint_batch(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_control::grant_role;
    use massa_sc_sdk::mock;

    #[test]
    fn minter_role() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc1155::constructor("u");
        mock::set_admin_context(false);
        let owner = context::caller();
        grant_role(MINTER_ROLE, &owner);
        entrypoints::mint(Args::new().add(&owner).add(U256::ONE).add(U256::from(3u64)).add(Vec::<u8>::new()));
        assert_eq!(internals::balance_of(&owner, U256::ONE), U256::from(3u64));
    }

    #[test]
    #[should_panic(expected = "Caller does not have minter role")]
    fn non_minter_rejected() {
        mock::reset();
        mock::set_deploy_context(None);
        crate::mrc1155::constructor("u");
        mock::set_admin_context(false);
        let owner = context::caller();
        entrypoints::mint(Args::new().add(&owner).add(U256::ONE).add(U256::ONE).add(Vec::<u8>::new()));
    }
}
