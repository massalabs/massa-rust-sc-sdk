//! Storage-level MRC20 primitives. None of them reconciles coins: compose them
//! inside your own entry point and call [`coins::transfer_remaining`] once at
//! its end (never nest two reconciliations in one execution).

use massa_sc_sdk::prelude::*;

/// Prefix of balance entries: `"BALANCE" ++ address`.
pub const BALANCE_KEY_PREFIX: &str = "BALANCE";
/// Prefix of allowance entries: `"ALLOWANCE" ++ owner ++ spender`.
pub const ALLOWANCE_KEY_PREFIX: &str = "ALLOWANCE";

/// Decode a little-endian `u256` from storage bytes (zero when malformed).
pub fn read_u256(bytes: &[u8]) -> U256 {
    if bytes.len() == 32 {
        let mut b = [0u8; 32];
        b.copy_from_slice(bytes);
        U256::from_le_bytes(b)
    } else {
        U256::ZERO
    }
}

/// Datastore key of the balance of `address`: `"BALANCE" ++ address`.
pub fn balance_key(address: &Address) -> Vec<u8> {
    let mut k = Vec::with_capacity(BALANCE_KEY_PREFIX.len() + address.as_bytes().len());
    k.extend_from_slice(BALANCE_KEY_PREFIX.as_bytes());
    k.extend_from_slice(address.as_bytes());
    k
}

/// Datastore key of the allowance of `spender` on `owner`:
/// `"ALLOWANCE" ++ owner ++ spender`.
pub fn allowance_key(owner: &Address, spender: &Address) -> Vec<u8> {
    let mut k =
        Vec::with_capacity(ALLOWANCE_KEY_PREFIX.len() + owner.as_bytes().len() + spender.as_bytes().len());
    k.extend_from_slice(ALLOWANCE_KEY_PREFIX.as_bytes());
    k.extend_from_slice(owner.as_bytes());
    k.extend_from_slice(spender.as_bytes());
    k
}

/// Token balance of `address` (0 when it has no entry).
pub fn balance(address: &Address) -> U256 {
    storage::try_get(balance_key(address)).map(|b| read_u256(&b)).unwrap_or(U256::ZERO)
}

/// Set the balance of `address`; a zero balance deletes the entry to reclaim
/// its storage.
pub fn set_balance(address: &Address, balance: U256) {
    let key = balance_key(address);
    if balance == U256::ZERO {
        storage::del_if_exists(key);
    } else {
        storage::set(key, balance.to_le_bytes());
    }
}

/// Move `amount` tokens from `from` to `to`. Aborts on insufficient funds or
/// overflow. A self-transfer is a no-op.
pub fn transfer(from: &Address, to: &Address, amount: U256) {
    let from_balance = balance(from);
    assert!(from_balance >= amount, "Transfer failed: insufficient funds");
    if from == to {
        return;
    }
    let to_balance = balance(to);
    let new_to = to_balance.checked_add(amount).expect("Transfer failed: overflow");
    set_balance(from, from_balance - amount);
    set_balance(to, new_to);
}

/// Set the allowance of `spender` on `owner`'s tokens.
pub fn approve(owner: &Address, spender: &Address, amount: U256) {
    storage::set(allowance_key(owner, spender), amount.to_le_bytes());
}

/// Allowance of `spender` on `owner`'s tokens (0 when unset).
pub fn allowance(owner: &Address, spender: &Address) -> U256 {
    storage::try_get(allowance_key(owner, spender)).map(|b| read_u256(&b)).unwrap_or(U256::ZERO)
}

/// Total supply stored by the token.
pub fn total_supply() -> U256 {
    read_u256(&storage::get(super::TOTAL_SUPPLY_KEY))
}

/// Overwrite the total supply.
pub fn set_total_supply(supply: U256) {
    storage::set(super::TOTAL_SUPPLY_KEY, supply.to_le_bytes());
}
