//! Helpers for contracts interacting with MRC20 tokens: storage costs a
//! caller must cover when a transfer creates a new balance entry.

use massa_sc_sdk::constants::{DATASTORE_ENTRY_BASE_SIZE, STORAGE_BYTE_COST};
use massa_sc_sdk::prelude::*;

use super::internals::{balance_key, BALANCE_KEY_PREFIX};

/// Storage cost (nanoMAS) of a fresh balance entry for an address whose
/// string form has `address_len` bytes: `(4 + key + 32-byte value) * byte cost`.
pub const fn balance_entry_cost(address_len: usize) -> u64 {
    (DATASTORE_ENTRY_BASE_SIZE + BALANCE_KEY_PREFIX.len() + address_len + 32) as u64 * STORAGE_BYTE_COST
}

/// Coins a caller must send with `transfer`/`transferFrom` on the token at
/// `token` so that the token can pay the balance entry of `recipient`;
/// `0` when the recipient already holds a balance.
pub fn get_balance_entry_cost(token: &Address, recipient: &Address) -> u64 {
    if storage::has_of(token, balance_key(recipient)) {
        0
    } else {
        balance_entry_cost(recipient.as_bytes().len())
    }
}

/// Storage cost of crediting `receiver` in *this* token's ledger, `0` when its
/// entry already exists.
pub fn compute_mint_storage_cost(receiver: &Address) -> u64 {
    if storage::has(balance_key(receiver)) {
        0
    } else {
        balance_entry_cost(receiver.as_bytes().len())
    }
}
