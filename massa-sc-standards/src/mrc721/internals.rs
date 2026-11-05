//! Storage-level MRC721 primitives (layout identical to
//! `@massalabs/sc-standards` `MRC721-internals`). `None` stands for "no
//! address" (mint source / burn destination), like `''` in AssemblyScript.

use massa_sc_sdk::prelude::*;

use crate::mrc20::internals::read_u256;

pub const NAME_KEY: &[u8] = &[0x01];
pub const SYMBOL_KEY: &[u8] = &[0x02];
pub const BALANCE_KEY_PREFIX: &[u8] = &[0x03];
pub const OWNER_KEY_PREFIX: &[u8] = &[0x04];
pub const ALLOWANCE_KEY_PREFIX: &[u8] = &[0x05];
pub const OPERATOR_ALLOWANCE_KEY_PREFIX: &[u8] = &[0x06];

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::with_capacity(parts.iter().map(|p| p.len()).sum());
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

pub fn constructor(name: &str, symbol: &str) {
    storage::set(NAME_KEY, name);
    storage::set(SYMBOL_KEY, symbol);
}

pub fn balance_key(owner: &Address) -> Vec<u8> {
    concat(&[BALANCE_KEY_PREFIX, owner.as_bytes()])
}

pub fn owner_key(token_id: U256) -> Vec<u8> {
    concat(&[OWNER_KEY_PREFIX, &token_id.to_le_bytes()])
}

pub fn allowance_key(token_id: U256) -> Vec<u8> {
    concat(&[ALLOWANCE_KEY_PREFIX, &token_id.to_le_bytes()])
}

pub fn operator_allowance_key(owner: &Address, operator: &Address) -> Vec<u8> {
    concat(&[OPERATOR_ALLOWANCE_KEY_PREFIX, owner.as_bytes(), operator.as_bytes()])
}

fn read_address(bytes: Vec<u8>) -> Address {
    Address::from(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn name() -> String {
    storage::try_get(NAME_KEY).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

pub fn symbol() -> String {
    storage::try_get(SYMBOL_KEY).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

/// Number of tokens held by `owner`.
pub fn balance_of(owner: &Address) -> U256 {
    storage::try_get(balance_key(owner)).map(|b| read_u256(&b)).unwrap_or(U256::ZERO)
}

/// Owner of `token_id`, `None` for a nonexistent token.
pub fn owner_of(token_id: U256) -> Option<Address> {
    storage::try_get(owner_key(token_id)).map(read_address)
}

/// Address approved for `token_id`, `None` when there is none.
pub fn get_approved(token_id: U256) -> Option<Address> {
    storage::try_get(allowance_key(token_id)).map(read_address)
}

pub fn is_approved(operator: &Address, token_id: U256) -> bool {
    get_approved(token_id).is_some_and(|a| &a == operator)
}

pub fn is_approved_for_all(owner: &Address, operator: &Address) -> bool {
    storage::try_get(operator_allowance_key(owner, operator)).is_some_and(|b| b.first() == Some(&1))
}

/// `true` when `operator` owns, is approved for, or is an operator of the
/// owner of `token_id`.
pub fn is_authorized(operator: &Address, token_id: U256) -> bool {
    match owner_of(token_id) {
        None => false,
        Some(owner) => {
            &owner == operator || is_approved(operator, token_id) || is_approved_for_all(&owner, operator)
        }
    }
}

/// Approve `approved` (or clear the approval with `None`) for `token_id`.
/// The caller must be authorized on the token.
pub fn approve(approved: Option<&Address>, token_id: U256) {
    assert!(owner_of(token_id).is_some(), "Nonexistent token");
    assert!(is_authorized(&context::caller(), token_id), "Unauthorized");
    let key = allowance_key(token_id);
    match approved {
        Some(a) => storage::set(key, a.as_bytes()),
        None => {
            storage::del_if_exists(key);
        }
    }
}

/// Grant or revoke `operator` on every token of the caller.
pub fn set_approval_for_all(operator: &Address, approved: bool) {
    assert!(!operator.as_str().is_empty(), "Invalid operator");
    let key = operator_allowance_key(&context::caller(), operator);
    if approved {
        storage::set(key, [1u8]);
    } else {
        storage::del(key);
    }
}

/// Move `token_id` to `to` (`None` = burn), from its current owner (`None` =
/// mint). When `auth` is given it must be authorized on the token. Returns the
/// previous owner.
pub fn update(to: Option<&Address>, token_id: U256, auth: Option<&Address>) -> Option<Address> {
    let from = owner_of(token_id);
    assert!(to != from.as_ref(), "The from and to addresses are the same");
    if let Some(auth) = auth {
        assert!(is_authorized(auth, token_id), "Unauthorized");
    }
    match &from {
        Some(from) => {
            // clear the approval
            approve(None, token_id);
            let from_key = balance_key(from);
            let from_balance = read_u256(&storage::get(&from_key));
            assert!(from_balance > U256::ZERO, "Insufficient balance");
            let new = from_balance - U256::ONE;
            if new == U256::ZERO {
                storage::del(&from_key);
            } else {
                storage::set(&from_key, new.to_le_bytes());
            }
        }
        None => {
            // a token with no owner must not keep an allowance from before it existed
            storage::del_if_exists(allowance_key(token_id));
        }
    }
    match to {
        Some(to) => {
            let to_key = balance_key(to);
            match storage::try_get(&to_key) {
                Some(b) => {
                    let bal = read_u256(&b);
                    assert!(bal < U256::MAX, "Balance overflow");
                    storage::set(&to_key, (bal + U256::ONE).to_le_bytes());
                }
                None => storage::set(&to_key, U256::ONE.to_le_bytes()),
            }
            storage::set(owner_key(token_id), to.as_bytes());
        }
        None => storage::del(owner_key(token_id)),
    }
    from
}

/// Transfer `token_id` from `from` to `to` on behalf of the caller.
pub fn transfer_from(from: &Address, to: &Address, token_id: U256) {
    assert!(is_authorized(&context::caller(), token_id), "Unauthorized caller");
    assert!(owner_of(token_id).as_ref() == Some(from), "Unauthorized from");
    assert!(!to.as_str().is_empty(), "Unauthorized to");
    assert!(storage::has(owner_key(token_id)), "Nonexistent token");
    update(Some(to), token_id, Some(from));
}
