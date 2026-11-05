//! MRC721 non-fungible token (Massa's ERC721), storage-compatible with
//! `@massalabs/sc-standards` `MRC721`.
//!
//! * [`internals`]: storage-level primitives.
//! * [`entrypoints`] + [`crate::export_mrc721!`]: the base standard
//!   (`name`, `symbol`, `version`, `balanceOf`, `ownerOf`, `getApproved`,
//!   `isApprovedForAll`, `approve`, `setApprovalForAll`, `transferFrom`,
//!   `mint` (owner only), `burn`); add [`crate::export_ownable!`] for
//!   `ownerAddress` / `setOwner` / `isOwner`.
//! * [`metadata`] + [`crate::export_mrc721_metadata!`]: `uri(tokenId)`.
//! * [`enumerable`] + [`crate::export_mrc721_enumerable!`]: total supply and
//!   per-owner token enumeration (replaces `mint`/`burn`/`transferFrom`).
//!
//! ```
//! use massa_sc_sdk::prelude::*;
//! use massa_sc_standards::{export_mrc721, export_mrc721_metadata, export_ownable, mrc721};
//!
//! #[export]
//! pub fn constructor(mut args: Args) {
//!     let name: String = args.must_next("name");
//!     let symbol: String = args.must_next("symbol");
//!     mrc721::constructor(&name, &symbol);
//! }
//! export_mrc721!();
//! export_mrc721_metadata!();
//! export_ownable!();
//! ```

pub mod enumerable;
pub mod internals;
pub mod metadata;

use massa_sc_sdk::prelude::*;

use crate::ownership;

/// Value returned by `version()`.
pub const VERSION: &str = "0.2.0";

/// Initialize the collection and make the caller its owner. Call it from
/// `constructor`; aborts outside of deployment.
pub fn constructor(name: &str, symbol: &str) {
    assert!(context::is_deploying_contract(), "constructor must be called at deployment");
    internals::constructor(name, symbol);
    ownership::set_owner(&context::caller());
}

pub use internals::{balance_of, get_approved, is_approved_for_all, name, owner_of, symbol};

/// Entry points taking serialized arguments (the on-chain interface).
pub mod entrypoints {
    use super::*;

    /// `name() -> string bytes`
    pub fn name() -> Vec<u8> {
        internals::name().into_bytes()
    }

    /// `symbol() -> string bytes`
    pub fn symbol() -> Vec<u8> {
        internals::symbol().into_bytes()
    }

    /// `version() -> string bytes`
    pub fn version() -> Vec<u8> {
        VERSION.as_bytes().to_vec()
    }

    /// `balanceOf(owner: string) -> u256`
    pub fn balance_of(mut args: Args) -> U256 {
        let owner: Address = args.must_next("address argument is missing or invalid");
        internals::balance_of(&owner)
    }

    /// `ownerOf(tokenId: u256) -> string bytes` (empty when nonexistent)
    pub fn owner_of(mut args: Args) -> Vec<u8> {
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::owner_of(token_id).map(|a| a.into_string().into_bytes()).unwrap_or_default()
    }

    /// `getApproved(tokenId: u256) -> string bytes` (empty when none)
    pub fn get_approved(mut args: Args) -> Vec<u8> {
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::get_approved(token_id).map(|a| a.into_string().into_bytes()).unwrap_or_default()
    }

    /// `isApprovedForAll(owner: string, operator: string) -> bool`
    pub fn is_approved_for_all(mut args: Args) -> bool {
        let owner: Address = args.must_next("owner argument is missing or invalid");
        let operator: Address = args.must_next("operator argument is missing or invalid");
        internals::is_approved_for_all(&owner, &operator)
    }

    /// `approve(to: string, tokenId: u256)`
    pub fn approve(mut args: Args) {
        let initial = coins::balance();
        let to: Address = args.must_next("to argument is missing or invalid");
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::approve(Some(&to), token_id);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `setApprovalForAll(operator: string, approved: bool)`
    pub fn set_approval_for_all(mut args: Args) {
        let initial = coins::balance();
        let operator: Address = args.must_next("to argument is missing or invalid");
        let approved: bool = args.must_next("approved argument is missing or invalid");
        internals::set_approval_for_all(&operator, approved);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `transferFrom(from: string, to: string, tokenId: u256)`
    pub fn transfer_from(mut args: Args) {
        let initial = coins::balance();
        let from: Address = args.must_next("from argument is missing or invalid");
        let to: Address = args.must_next("to argument is missing or invalid");
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::transfer_from(&from, &to, token_id);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `mint(to: string, tokenId: u256)`, owner only.
    pub fn mint(mut args: Args) {
        let initial = coins::balance();
        ownership::only_owner();
        let to: Address = args.must_next("to argument is missing or invalid");
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::update(Some(&to), token_id, None);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `burn(tokenId: u256)`: anyone authorized on the token may burn it.
    pub fn burn(mut args: Args) {
        let initial = coins::balance();
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        internals::update(None, token_id, Some(&context::caller()));
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export the read-only and approval entry points shared by the base and
/// enumerable variants.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_mrc721_common {
    () => {
        #[::massa_sc_sdk::export(name = "name")]
        pub fn __std_name() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc721::entrypoints::name()
        }
        #[::massa_sc_sdk::export(name = "symbol")]
        pub fn __std_symbol() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc721::entrypoints::symbol()
        }
        #[::massa_sc_sdk::export(name = "version")]
        pub fn __std_version() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc721::entrypoints::version()
        }
        #[::massa_sc_sdk::export(name = "balanceOf")]
        pub fn __std_balance_of(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::U256 {
            $crate::mrc721::entrypoints::balance_of(args)
        }
        #[::massa_sc_sdk::export(name = "ownerOf")]
        pub fn __std_owner_of(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc721::entrypoints::owner_of(args)
        }
        #[::massa_sc_sdk::export(name = "getApproved")]
        pub fn __std_get_approved(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::mrc721::entrypoints::get_approved(args)
        }
        #[::massa_sc_sdk::export(name = "isApprovedForAll")]
        pub fn __std_is_approved_for_all(args: ::massa_sc_sdk::Args) -> bool {
            $crate::mrc721::entrypoints::is_approved_for_all(args)
        }
        #[::massa_sc_sdk::export(name = "approve")]
        pub fn __std_approve(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::entrypoints::approve(args)
        }
        #[::massa_sc_sdk::export(name = "setApprovalForAll")]
        pub fn __std_set_approval_for_all(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::entrypoints::set_approval_for_all(args)
        }
    };
}

/// Export the base MRC721 entry points.
#[macro_export]
macro_rules! export_mrc721 {
    () => {
        $crate::__export_mrc721_common!();
        #[::massa_sc_sdk::export(name = "transferFrom")]
        pub fn __std_transfer_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::entrypoints::transfer_from(args)
        }
        #[::massa_sc_sdk::export(name = "mint")]
        pub fn __std_mint(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::entrypoints::mint(args)
        }
        #[::massa_sc_sdk::export(name = "burn")]
        pub fn __std_burn(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::entrypoints::burn(args)
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
        constructor("Massa NFT", "MNFT");
        mock::set_admin_context(false);
        context::caller()
    }

    #[test]
    fn mint_transfer_approve_burn() {
        let owner = deploy();
        assert_eq!(name(), "Massa NFT");
        assert_eq!(symbol(), "MNFT");
        let id = U256::from(7u64);
        entrypoints::mint(Args::new().add(&owner).add(id));
        assert_eq!(owner_of(id), Some(owner.clone()));
        assert_eq!(balance_of(&owner), U256::ONE);
        assert_eq!(entrypoints::owner_of(Args::new().add(id)), owner.as_bytes());

        let bob = mock::new_user_address();
        entrypoints::approve(Args::new().add(&bob).add(id));
        assert_eq!(get_approved(id), Some(bob.clone()));
        mock::set_caller(&bob);
        let carol = mock::new_user_address();
        entrypoints::transfer_from(Args::new().add(&owner).add(&carol).add(id));
        assert_eq!(owner_of(id), Some(carol.clone()));
        assert_eq!(get_approved(id), None, "approval cleared on transfer");
        assert_eq!(balance_of(&owner), U256::ZERO);
        assert!(!storage::has(internals::balance_key(&owner)));

        mock::set_caller(&carol);
        entrypoints::set_approval_for_all(Args::new().add(&bob).add(true));
        assert!(is_approved_for_all(&carol, &bob));
        mock::set_caller(&bob);
        entrypoints::burn(Args::new().add(id));
        assert_eq!(owner_of(id), None);
        assert_eq!(balance_of(&carol), U256::ZERO);
    }

    #[test]
    #[should_panic(expected = "Unauthorized caller")]
    fn transfer_requires_authorization() {
        let owner = deploy();
        let id = U256::ONE;
        entrypoints::mint(Args::new().add(&owner).add(id));
        mock::set_caller(&mock::new_user_address());
        entrypoints::transfer_from(Args::new().add(&owner).add(mock::new_user_address()).add(id));
    }

    #[test]
    #[should_panic(expected = "Caller is not the owner")]
    fn only_owner_mints() {
        let _ = deploy();
        mock::set_caller(&mock::new_user_address());
        entrypoints::mint(Args::new().add(context::caller()).add(U256::ONE));
    }
}
