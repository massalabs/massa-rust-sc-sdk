//! MRC1155 multi-token (Massa's ERC1155), storage-compatible with
//! `@massalabs/sc-standards` `MRC1155`.
//!
//! * [`internals`]: storage-level primitives.
//! * [`entrypoints`] + [`crate::export_mrc1155!`]: `version`, `uri`,
//!   `balanceOf`, `balanceOfBatch`, `setApprovalForAll`, `isApprovedForAll`,
//!   `safeTransferFrom`, `safeBatchTransferFrom`.
//! * [`metadata`] + [`crate::export_mrc1155_metadata!`]: the same set with
//!   per-token URIs (use one of the two macros).
//! * [`mintable`] + [`crate::export_mrc1155_mintable!`]: `mint`, `mintBatch`
//!   (role `minter`, see [`crate::access_control`]).
//! * [`burnable`] + [`crate::export_mrc1155_burnable!`]: `burn`, `burnBatch`.

pub mod burnable;
pub mod internals;
pub mod metadata;
pub mod mintable;

use massa_sc_sdk::prelude::*;

use crate::ownership;

pub const VERSION: &str = "0.1.0";

/// Initialize the collection URI and make the caller its owner. Call it from
/// `constructor`; aborts outside of deployment.
pub fn constructor(uri: &str) {
    assert!(context::is_deploying_contract(), "constructor must be called at deployment");
    ownership::set_owner(&context::caller());
    internals::constructor(uri);
}

pub use internals::{balance_of, balance_of_batch, is_approved_for_all, uri};

pub mod entrypoints {
    use super::*;

    pub fn version() -> Vec<u8> {
        VERSION.as_bytes().to_vec()
    }

    /// `uri(id: u256) -> string bytes`
    pub fn uri(mut args: Args) -> String {
        let id: U256 = args.must_next("id argument is missing or invalid");
        internals::uri(id)
    }

    /// `balanceOf(owner: string, id: u256) -> u256`
    pub fn balance_of(mut args: Args) -> U256 {
        let owner: Address = args.must_next("owner argument is missing or invalid");
        let id: U256 = args.must_next("id argument is missing or invalid");
        internals::balance_of(&owner, id)
    }

    /// `balanceOfBatch(owners: string[], ids: u256[]) -> u256[]` (packed, no length prefix)
    pub fn balance_of_batch(mut args: Args) -> Vec<u8> {
        let owners: Vec<Address> = args.must_next("owners argument is missing or invalid");
        let ids: Vec<U256> = args.must_next("ids argument is missing or invalid");
        assert!(owners.len() == ids.len(), "{}", internals::INVALID_ARRAY_LENGTH_ERROR);
        let balances = internals::balance_of_batch(&owners, &ids);
        let mut out = Vec::with_capacity(balances.len() * 32);
        for b in balances {
            out.extend_from_slice(&b.to_le_bytes());
        }
        out
    }

    /// `setApprovalForAll(operator: string, approved: bool)`
    pub fn set_approval_for_all(mut args: Args) {
        let initial = coins::balance();
        let sender = context::caller();
        let operator: Address = args.must_next("operator argument is missing or invalid");
        let approved: bool = args.must_next("approved argument is missing or invalid");
        internals::set_approval_for_all(&sender, &operator, approved);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `isApprovedForAll(owner: string, operator: string) -> bool`
    pub fn is_approved_for_all(mut args: Args) -> bool {
        let owner: Address = args.must_next("owner argument is missing or invalid");
        let operator: Address = args.must_next("operator argument is missing or invalid");
        internals::is_approved_for_all(&owner, &operator)
    }

    /// `safeTransferFrom(from: string, to: string, id: u256, value: u256, data: bytes)`
    pub fn safe_transfer_from(mut args: Args) {
        let initial = coins::balance();
        let sender = context::caller();
        let from: Address = args.must_next("from argument is missing or invalid");
        let to: Address = args.must_next("to argument is missing or invalid");
        let id: U256 = args.must_next("id argument is missing or invalid");
        let value: U256 = args.must_next("value argument is missing or invalid");
        let data: Vec<u8> = args.must_next("data argument is missing or invalid");
        assert!(
            from == sender || internals::is_approved_for_all(&from, &sender),
            "{}",
            internals::MISSING_APPROVAL_FOR_ALL_ERROR
        );
        internals::safe_transfer_from(&from, &to, id, value, &data);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `safeBatchTransferFrom(from: string, to: string, ids: u256[], values: u256[], data: bytes)`
    pub fn safe_batch_transfer_from(mut args: Args) {
        let initial = coins::balance();
        let sender = context::caller();
        let from: Address = args.must_next("from argument is missing or invalid");
        let to: Address = args.must_next("to argument is missing or invalid");
        let ids: Vec<U256> = args.must_next("ids argument is missing or invalid");
        let values: Vec<U256> = args.must_next("values argument is missing or invalid");
        let data: Vec<u8> = args.must_next("data argument is missing or invalid");
        assert!(
            from == sender || internals::is_approved_for_all(&from, &sender),
            "{}",
            internals::MISSING_APPROVAL_FOR_ALL_ERROR
        );
        internals::safe_batch_transfer_from(&from, &to, &ids, &values, &data);
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Entry points shared by the base and metadata variants (everything but `uri`).
#[doc(hidden)]
#[macro_export]
macro_rules! __export_mrc1155_common {
    () => {
        #[::massa_sc_sdk::export(name = "version")]
        pub fn __std_version() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc1155::entrypoints::version()
        }
        #[::massa_sc_sdk::export(name = "balanceOf")]
        pub fn __std_balance_of(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::U256 {
            $crate::mrc1155::entrypoints::balance_of(args)
        }
        #[::massa_sc_sdk::export(name = "balanceOfBatch")]
        pub fn __std_balance_of_batch(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc1155::entrypoints::balance_of_batch(args)
        }
        #[::massa_sc_sdk::export(name = "setApprovalForAll")]
        pub fn __std_set_approval_for_all(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::entrypoints::set_approval_for_all(args)
        }
        #[::massa_sc_sdk::export(name = "isApprovedForAll")]
        pub fn __std_is_approved_for_all(args: ::massa_sc_sdk::Args) -> bool {
            $crate::mrc1155::entrypoints::is_approved_for_all(args)
        }
        #[::massa_sc_sdk::export(name = "safeTransferFrom")]
        pub fn __std_safe_transfer_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::entrypoints::safe_transfer_from(args)
        }
        #[::massa_sc_sdk::export(name = "safeBatchTransferFrom")]
        pub fn __std_safe_batch_transfer_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc1155::entrypoints::safe_batch_transfer_from(args)
        }
    };
}

/// Export the base MRC1155 entry points (`version`, `uri`, `balanceOf`,
/// `balanceOfBatch`, `setApprovalForAll`, `isApprovedForAll`,
/// `safeTransferFrom`, `safeBatchTransferFrom`). Use
/// [`crate::export_mrc1155_metadata!`] *instead* for per-token URIs.
#[macro_export]
macro_rules! export_mrc1155 {
    () => {
        $crate::__export_mrc1155_common!();
        #[::massa_sc_sdk::export(name = "uri")]
        pub fn __std_uri(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::String {
            $crate::mrc1155::entrypoints::uri(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    fn deploy() -> Address {
        mock::reset();
        mock::set_deploy_context(None);
        constructor("ipfs://collection");
        mock::set_admin_context(false);
        mock::clear_events();
        context::caller()
    }

    #[test]
    fn transfers_and_batches() {
        let owner = deploy();
        internals::mint(&owner, U256::ONE, U256::from(10u64), &[]);
        internals::mint_batch(&owner, &[U256::from(2u64), U256::from(3u64)], &[U256::from(5u64); 2], &[]);
        assert_eq!(balance_of(&owner, U256::ONE), U256::from(10u64));
        assert_eq!(
            balance_of_batch(&[owner.clone(), owner.clone()], &[U256::from(2u64), U256::from(3u64)]),
            vec![U256::from(5u64); 2]
        );
        assert_eq!(uri(U256::ONE), "ipfs://collection");

        let bob = mock::new_user_address();
        entrypoints::safe_transfer_from(
            Args::new().add(&owner).add(&bob).add(U256::ONE).add(U256::from(4u64)).add(Vec::<u8>::new()),
        );
        assert_eq!(balance_of(&bob, U256::ONE), U256::from(4u64));
        assert_eq!(balance_of(&owner, U256::ONE), U256::from(6u64));

        entrypoints::set_approval_for_all(Args::new().add(&bob).add(true));
        assert!(is_approved_for_all(&owner, &bob));
        mock::set_caller(&bob);
        entrypoints::safe_batch_transfer_from(
            Args::new()
                .add(&owner)
                .add(&bob)
                .add(vec![U256::from(2u64), U256::from(3u64)])
                .add(vec![U256::from(5u64), U256::from(1u64)])
                .add(Vec::<u8>::new()),
        );
        assert_eq!(balance_of(&bob, U256::from(2u64)), U256::from(5u64));
        assert_eq!(balance_of(&owner, U256::from(2u64)), U256::ZERO);
        assert!(!storage::has(internals::balance_key(U256::from(2u64), &owner)));
        let events = mock::events();
        assert!(events.iter().any(|e| e.starts_with("TransferBatch:")));
        assert!(events.iter().any(|e| e.starts_with("TransferSingle:")));
    }

    #[test]
    fn receiver_hook_is_checked() {
        let owner = deploy();
        internals::mint(&owner, U256::ONE, U256::from(10u64), &[]);
        let receiver = mock::new_sc_address();
        mock::mock_call(&receiver, "OnMRC1155Received", Args::new().add("f23a6e61").into_bytes());
        internals::safe_transfer_from(&owner, &receiver, U256::ONE, U256::ONE, &[1, 2]);
        let call = &mock::recorded_calls()[0];
        assert_eq!(call.function, "OnMRC1155Received");
        assert_eq!(balance_of(&receiver, U256::ONE), U256::ONE);
    }

    #[test]
    #[should_panic(expected = "MRC1155MissingApprovalForAll")]
    fn transfer_needs_approval() {
        let owner = deploy();
        internals::mint(&owner, U256::ONE, U256::from(10u64), &[]);
        mock::set_caller(&mock::new_user_address());
        entrypoints::safe_transfer_from(
            Args::new()
                .add(&owner)
                .add(context::caller())
                .add(U256::ONE)
                .add(U256::ONE)
                .add(Vec::<u8>::new()),
        );
    }
}
