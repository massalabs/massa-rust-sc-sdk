//! MRC721 enumerable extension: total supply and per-owner token enumeration
//! (one empty datastore entry `ownedTokens ++ owner ++ tokenId` per token, so
//! an owner's tokens can be listed with a prefixed key query).

use massa_sc_sdk::prelude::*;

use super::internals::{self, is_authorized, owner_key};
use crate::mrc20::internals::read_u256;
use crate::ownership;

pub use super::internals::owner_of;
pub use super::internals::{balance_of, get_approved, is_approved_for_all, name, symbol};

/// Datastore key of the total supply (`u256`, little-endian).
pub const TOTAL_SUPPLY_KEY: &str = "totalSupply";
/// Prefix of enumeration entries: `"ownedTokens" ++ owner ++ tokenId`.
pub const OWNED_TOKENS_KEY: &str = "ownedTokens";

/// Initialize the collection (metadata, zero supply) and make the caller its
/// owner. Call it from `constructor`; aborts outside of deployment.
pub fn constructor(name: &str, symbol: &str) {
    assert!(context::is_deploying_contract(), "constructor must be called at deployment");
    internals::constructor(name, symbol);
    storage::set(TOTAL_SUPPLY_KEY, U256::ZERO.to_le_bytes());
    ownership::set_owner(&context::caller());
}

/// Number of existing tokens.
pub fn total_supply() -> U256 {
    read_u256(&storage::get(TOTAL_SUPPLY_KEY))
}

fn increase_total_supply(delta: U256) {
    let new = total_supply().checked_add(delta).expect("Total supply overflow");
    storage::set(TOTAL_SUPPLY_KEY, new.to_le_bytes());
}

fn decrease_total_supply(delta: U256) {
    let new = total_supply().checked_sub(delta).expect("Total supply underflow");
    storage::set(TOTAL_SUPPLY_KEY, new.to_le_bytes());
}

/// Key prefix under which the tokens of `owner` are enumerated.
pub fn owned_tokens_prefix(owner: &Address) -> Vec<u8> {
    let mut k = Vec::with_capacity(OWNED_TOKENS_KEY.len() + owner.as_bytes().len());
    k.extend_from_slice(OWNED_TOKENS_KEY.as_bytes());
    k.extend_from_slice(owner.as_bytes());
    k
}

fn owned_token_key(owner: &Address, token_id: U256) -> Vec<u8> {
    let mut k = owned_tokens_prefix(owner);
    k.extend_from_slice(&token_id.to_le_bytes());
    k
}

/// Token ids owned by `owner`, in ascending order.
///
/// Uses the legacy key query, which is available on every network (Massa and
/// C-Layer) but fails when the owner holds more than
/// [`massa_sc_sdk::constants::MAX_DATASTORE_KEYS_PAGE`] tokens from
/// MIP-0002 on; use [`owned_tokens_page`] for collections where that can
/// happen.
pub fn owned_tokens(owner: &Address) -> Vec<U256> {
    let prefix = owned_tokens_prefix(owner);
    storage::get_keys(&prefix).into_iter().map(|k| read_u256(&k[prefix.len()..])).collect()
}

/// One page of token ids owned by `owner`, strictly after `after`
/// (`None` = from the first), at most `count` (`1..=MAX_DATASTORE_KEYS_PAGE`).
///
/// Requires execution version >= 2 (MIP-0002): a module calling it cannot be
/// instantiated on nodes without it (e.g. C-Layer at the time of writing).
pub fn owned_tokens_page(owner: &Address, after: Option<U256>, count: u32) -> Vec<U256> {
    let prefix = owned_tokens_prefix(owner);
    let start = after.map(|id| owned_token_key(owner, id)).unwrap_or_default();
    storage::get_keys_page(&prefix, start, count).into_iter().map(|k| read_u256(&k[prefix.len()..])).collect()
}

/// [`internals::update`] plus enumeration and supply bookkeeping.
pub fn update(to: Option<&Address>, token_id: U256, auth: Option<&Address>) -> Option<Address> {
    let previous = internals::update(to, token_id, auth);
    match (&previous, to) {
        (None, Some(to)) => {
            storage::set(owned_token_key(to, token_id), []);
            increase_total_supply(U256::ONE);
        }
        (Some(prev), Some(to)) => {
            if prev != to {
                storage::del(owned_token_key(prev, token_id));
                storage::set(owned_token_key(to, token_id), []);
            }
        }
        (Some(prev), None) => {
            storage::del(owned_token_key(prev, token_id));
            decrease_total_supply(U256::ONE);
        }
        (None, None) => {}
    }
    previous
}

/// Transfer `token_id` from `from` to `to` on behalf of the caller, keeping the enumeration up to date.
pub fn transfer_from(from: &Address, to: &Address, token_id: U256) {
    assert!(is_authorized(&context::caller(), token_id), "Unauthorized caller");
    assert!(owner_of(token_id).as_ref() == Some(from), "Unauthorized from");
    assert!(!to.as_str().is_empty(), "Unauthorized to");
    assert!(storage::has(owner_key(token_id)), "Nonexistent token");
    update(Some(to), token_id, Some(from));
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `totalSupply() -> u256`
    pub fn total_supply() -> U256 {
        super::total_supply()
    }

    /// `transferFrom(from: string, to: string, tokenId: u256)`
    pub fn transfer_from(mut args: Args) {
        let initial = coins::balance();
        let from: Address = args.must_next("from argument is missing or invalid");
        let to: Address = args.must_next("to argument is missing or invalid");
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        super::transfer_from(&from, &to, token_id);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `mint(to: string, tokenId: u256)`, owner only.
    pub fn mint(mut args: Args) {
        let initial = coins::balance();
        ownership::only_owner();
        let to: Address = args.must_next("to argument is missing or invalid");
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        update(Some(&to), token_id, None);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `burn(tokenId: u256)`: anyone authorized on the token may burn it.
    pub fn burn(mut args: Args) {
        let initial = coins::balance();
        let token_id: U256 = args.must_next("tokenId argument is missing or invalid");
        update(None, token_id, Some(&context::caller()));
        coins::transfer_remaining(initial, 0, None);
    }
}

/// Export the enumerable MRC721 entry points (base set + `totalSupply`,
/// with enumeration-aware `transferFrom`, `mint` and `burn`).
#[macro_export]
macro_rules! export_mrc721_enumerable {
    () => {
        $crate::__export_mrc721_common!();
        #[::massa_sc_sdk::export(name = "totalSupply")]
        pub fn __std_total_supply() -> ::massa_sc_sdk::U256 {
            $crate::mrc721::enumerable::entrypoints::total_supply()
        }
        #[::massa_sc_sdk::export(name = "transferFrom")]
        pub fn __std_transfer_from(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::enumerable::entrypoints::transfer_from(args)
        }
        #[::massa_sc_sdk::export(name = "mint")]
        pub fn __std_mint(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::enumerable::entrypoints::mint(args)
        }
        #[::massa_sc_sdk::export(name = "burn")]
        pub fn __std_burn(args: ::massa_sc_sdk::Args) {
            $crate::mrc721::enumerable::entrypoints::burn(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn supply_and_enumeration() {
        mock::reset();
        mock::set_deploy_context(None);
        constructor("E", "E");
        mock::set_admin_context(false);
        let owner = context::caller();
        for i in 1..=3u64 {
            entrypoints::mint(Args::new().add(&owner).add(U256::from(i)));
        }
        assert_eq!(total_supply(), U256::from(3u64));
        assert_eq!(owned_tokens(&owner).len(), 3);
        assert_eq!(owned_tokens_page(&owner, Some(U256::ONE), 10), vec![U256::from(2u64), U256::from(3u64)]);
        let bob = mock::new_user_address();
        entrypoints::transfer_from(Args::new().add(&owner).add(&bob).add(U256::from(2u64)));
        assert_eq!(owned_tokens(&owner).len(), 2);
        assert_eq!(owned_tokens(&bob), vec![U256::from(2u64)]);
        mock::set_caller(&bob);
        entrypoints::burn(Args::new().add(U256::from(2u64)));
        assert_eq!(total_supply(), U256::from(2u64));
        assert!(owned_tokens(&bob).is_empty());
    }
}
