//! Storage-level MRC1155 primitives (layout identical to
//! `@massalabs/sc-standards` `MRC1155-internal`). `None` stands for "no
//! address" (mint source / burn destination).

use massa_sc_sdk::prelude::*;

use crate::mrc20::internals::read_u256;

pub const URI_KEY: &[u8] = &[0x01];
pub const BALANCE_KEY_PREFIX: &[u8] = &[0x02];
pub const OPERATOR_APPROVAL_KEY_PREFIX: &[u8] = &[0x03];

pub const APPROVAL_FOR_ALL_EVENT: &str = "ApprovalForAll";
pub const TRANSFER_SINGLE_EVENT: &str = "TransferSingle";
pub const TRANSFER_BATCH_EVENT: &str = "TransferBatch";
pub const URI_EVENT: &str = "URI";

pub const INVALID_OPERATOR_ERROR: &str = "InvalidOperator";
pub const BALANCE_OVERFLOW_ERROR: &str = "MRC1155BalanceOverflow";
pub const INSUFFICIENT_BALANCE_ERROR: &str = "MRC1155InsufficientBalance";
pub const INVALID_ARRAY_LENGTH_ERROR: &str = "MRC1155InvalidArrayLength";
pub const INVALID_RECEIVER_ERROR: &str = "MRC1155InvalidReceiver";
pub const INVALID_SENDER_ERROR: &str = "MRC1155InvalidSender";
pub const MISSING_APPROVAL_FOR_ALL_ERROR: &str = "MRC1155MissingApprovalForAll";

/// Magic value a receiving contract must return from `OnMRC1155Received`.
pub const ON_RECEIVED_MAGIC: &str = "f23a6e61";
/// Magic value a receiving contract must return from `onMRC1155BatchReceived`.
pub const ON_BATCH_RECEIVED_MAGIC: &str = "bc197c81";

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::with_capacity(parts.iter().map(|p| p.len()).sum());
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

pub fn constructor(uri: &str) {
    set_uri(uri);
}

pub fn balance_key(id: U256, address: &Address) -> Vec<u8> {
    concat(&[BALANCE_KEY_PREFIX, &id.to_le_bytes(), address.as_bytes()])
}

pub fn operator_approval_key(owner: &Address, operator: &Address) -> Vec<u8> {
    concat(&[OPERATOR_APPROVAL_KEY_PREFIX, owner.as_bytes(), operator.as_bytes()])
}

pub fn balance_of(owner: &Address, id: U256) -> U256 {
    storage::try_get(balance_key(id, owner)).map(|b| read_u256(&b)).unwrap_or(U256::ZERO)
}

pub fn balance_of_batch(owners: &[Address], ids: &[U256]) -> Vec<U256> {
    assert!(owners.len() == ids.len(), "{INVALID_ARRAY_LENGTH_ERROR}");
    owners.iter().zip(ids).map(|(o, &id)| balance_of(o, id)).collect()
}

/// Collection-wide URI (the base standard ignores `id`).
pub fn uri(_id: U256) -> String {
    storage::try_get(URI_KEY).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

pub fn set_uri(new_uri: &str) {
    storage::set(URI_KEY, new_uri);
    generate_event(&create_event(URI_EVENT, &[new_uri, new_uri]));
}

pub fn set_approval_for_all(owner: &Address, operator: &Address, approved: bool) {
    assert!(!operator.as_str().is_empty(), "{INVALID_OPERATOR_ERROR}");
    let key = operator_approval_key(owner, operator);
    if approved {
        storage::set(key, [1u8]);
    } else {
        storage::del(key);
    }
    generate_event(&create_event(
        APPROVAL_FOR_ALL_EVENT,
        &[owner.as_str(), operator.as_str(), if approved { "true" } else { "false" }],
    ));
}

pub fn is_approved_for_all(owner: &Address, operator: &Address) -> bool {
    storage::try_get(operator_approval_key(owner, operator)).is_some_and(|b| b.first() == Some(&1))
}

fn join(values: &[U256]) -> String {
    let mut s = String::new();
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            s.push(';');
        }
        s.push_str(&v.to_string());
    }
    s
}

fn addr_or_empty(a: Option<&Address>) -> &str {
    a.map(|a| a.as_str()).unwrap_or("")
}

/// Move `values[i]` units of `ids[i]` from `from` (`None` = mint) to `to`
/// (`None` = burn) and emit the transfer event.
pub fn update(from: Option<&Address>, to: Option<&Address>, ids: &[U256], values: &[U256]) {
    assert!(ids.len() == values.len(), "{INVALID_ARRAY_LENGTH_ERROR}");
    let operator = context::caller();
    for (&id, &value) in ids.iter().zip(values) {
        if let Some(from) = from {
            let key = balance_key(id, from);
            let bal = storage::try_get(&key).map(|b| read_u256(&b)).unwrap_or(U256::ZERO);
            assert!(bal >= value, "{INSUFFICIENT_BALANCE_ERROR}");
            let new = bal - value;
            if new == U256::ZERO {
                storage::del_if_exists(&key);
            } else {
                storage::set(&key, new.to_le_bytes());
            }
        }
        if let Some(to) = to {
            let key = balance_key(id, to);
            let bal = storage::try_get(&key).map(|b| read_u256(&b)).unwrap_or(U256::ZERO);
            let new = bal.checked_add(value).expect(BALANCE_OVERFLOW_ERROR);
            if new != U256::ZERO {
                storage::set(&key, new.to_le_bytes());
            }
        }
    }
    if ids.len() == 1 {
        generate_event(&create_event(
            TRANSFER_SINGLE_EVENT,
            &[
                operator.as_str(),
                addr_or_empty(from),
                addr_or_empty(to),
                &ids[0].to_string(),
                &values[0].to_string(),
            ],
        ));
    } else {
        generate_event(&create_event(
            TRANSFER_BATCH_EVENT,
            &[operator.as_str(), addr_or_empty(from), addr_or_empty(to), &join(ids), &join(values)],
        ));
    }
}

/// [`update`] then, when `to` is a contract exporting the receiver hook,
/// call it and check the returned magic value.
pub fn update_with_acceptance_check(
    from: Option<&Address>,
    to: Option<&Address>,
    ids: &[U256],
    values: &[U256],
    data: &[u8],
) {
    update(from, to, ids, values);
    let Some(to) = to else { return };
    if !to.as_str().starts_with("AS") {
        return;
    }
    let operator = context::caller();
    let from_str = addr_or_empty(from);
    if ids.len() == 1 {
        if contract::function_exists(to, "OnMRC1155Received") {
            let output = contract::call(
                to,
                "OnMRC1155Received",
                Args::new().add(&operator).add(from_str).add(ids[0]).add(values[0]).add(data),
                0,
            );
            assert!(output == Args::new().add(ON_RECEIVED_MAGIC).into_bytes(), "MRC1155InvalidReceiver");
        }
    } else if contract::function_exists(to, "onMRC1155BatchReceived") {
        let output = contract::call(
            to,
            "onMRC1155BatchReceived",
            Args::new().add(&operator).add(from_str).add(ids).add(values).add(data),
            0,
        );
        assert!(output == Args::new().add(ON_BATCH_RECEIVED_MAGIC).into_bytes(), "MRC1155InvalidReceiver");
    }
}

pub fn safe_transfer_from(from: &Address, to: &Address, id: U256, value: U256, data: &[u8]) {
    assert!(!to.as_str().is_empty(), "{INVALID_RECEIVER_ERROR}");
    assert!(!from.as_str().is_empty(), "{INVALID_SENDER_ERROR}");
    update_with_acceptance_check(Some(from), Some(to), &[id], &[value], data);
}

pub fn safe_batch_transfer_from(from: &Address, to: &Address, ids: &[U256], values: &[U256], data: &[u8]) {
    assert!(!to.as_str().is_empty(), "{INVALID_RECEIVER_ERROR}");
    assert!(!from.as_str().is_empty(), "{INVALID_SENDER_ERROR}");
    update_with_acceptance_check(Some(from), Some(to), ids, values, data);
}

pub fn mint(to: &Address, id: U256, value: U256, data: &[u8]) {
    assert!(!to.as_str().is_empty(), "{INVALID_RECEIVER_ERROR}");
    update_with_acceptance_check(None, Some(to), &[id], &[value], data);
}

pub fn mint_batch(to: &Address, ids: &[U256], values: &[U256], data: &[u8]) {
    assert!(!to.as_str().is_empty(), "{INVALID_RECEIVER_ERROR}");
    update_with_acceptance_check(None, Some(to), ids, values, data);
}

pub fn burn(from: &Address, id: U256, value: U256) {
    assert!(!from.as_str().is_empty(), "{INVALID_SENDER_ERROR}");
    update_with_acceptance_check(Some(from), None, &[id], &[value], &[]);
}

pub fn burn_batch(from: &Address, ids: &[U256], values: &[U256]) {
    assert!(!from.as_str().is_empty(), "{INVALID_SENDER_ERROR}");
    update_with_acceptance_check(Some(from), None, ids, values, &[]);
}
