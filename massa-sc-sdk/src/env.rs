//! Thin, safe, typed wrappers over every host ABI. This is the lowest level
//! most contracts should ever need; the `storage`, `context`, `coins`, ...
//! modules build on it with friendlier types (e.g. [`crate::Address`]).
//!
//! On `wasm32` these call the real host imports. On any other target they are
//! served by the in-memory [`crate::mock`] VM, so contract logic can be unit
//! tested natively with `cargo test`.

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(target_arch = "wasm32")]
use crate::abi::massa as abi;
#[cfg(target_arch = "wasm32")]
use crate::memory::{take_bytes, take_hash, take_string, HostBuf};
#[cfg(not(target_arch = "wasm32"))]
use crate::mock::with_vm;

pub use crate::codec::{decode_keys, json_string_array as decode_json_strings};

// ---------------------------------------------------------------------------
// logging / events
// ---------------------------------------------------------------------------

/// Print a message in the node logs (no on-chain effect).
pub fn print(message: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_print(HostBuf::string(message).ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.print(message))
}

/// Emit an event, stored on chain with the execution output.
pub fn generate_event(event: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_generate_event(HostBuf::string(event).ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.generate_event(event))
}

// ---------------------------------------------------------------------------
// calls & bytecode
// ---------------------------------------------------------------------------

/// Call `function` of the contract at `address`, transferring `coins` nanoMAS.
pub fn call(address: &str, function: &str, param: &[u8], coins: u64) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let f = HostBuf::string(function);
        let p = HostBuf::bytes(param);
        take_bytes(abi::assembly_script_call(a.ptr(), f.ptr(), p.ptr(), coins as i64))
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::mock::call(address, function, param, coins)
}

/// Run `function` of the bytecode stored at `address` in the *current* context.
pub fn local_call(address: &str, function: &str, param: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let f = HostBuf::string(function);
        let p = HostBuf::bytes(param);
        take_bytes(abi::assembly_script_local_call(a.ptr(), f.ptr(), p.ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::mock::local_call(address, function, param)
}

/// Run `function` of the given `bytecode` in the *current* context.
pub fn local_execution(bytecode: &[u8], function: &str, param: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let b = HostBuf::bytes(bytecode);
        let f = HostBuf::string(function);
        let p = HostBuf::bytes(param);
        take_bytes(abi::assembly_script_local_execution(b.ptr(), f.ptr(), p.ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::mock::local_execution(bytecode, function, param)
}

/// Deploy `bytecode` at a fresh address and return that address.
pub fn create_sc(bytecode: &[u8]) -> String {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_string(abi::assembly_script_create_sc(HostBuf::bytes(bytecode).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.create_sc(bytecode))
}

pub fn get_bytecode() -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_bytes(abi::assembly_script_get_bytecode())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_bytecode())
}

pub fn get_bytecode_of(address: &str) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_bytes(abi::assembly_script_get_bytecode_for(HostBuf::string(address).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_bytecode_of(address))
}

pub fn set_bytecode(bytecode: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_set_bytecode(HostBuf::bytes(bytecode).ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.set_bytecode(bytecode))
}

pub fn set_bytecode_of(address: &str, bytecode: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let b = HostBuf::bytes(bytecode);
        abi::assembly_script_set_bytecode_for(a.ptr(), b.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.set_bytecode_of(address, bytecode))
}

pub fn function_exists(address: &str, function: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let f = HostBuf::string(function);
        abi::assembly_script_function_exists(a.ptr(), f.ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.function_exists(address, function))
}

pub fn caller_has_write_access() -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_caller_has_write_access() != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.caller_has_write_access())
}

pub fn remaining_gas() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_remaining_gas() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.remaining_gas())
}

// ---------------------------------------------------------------------------
// datastore
// ---------------------------------------------------------------------------

pub fn set(key: &[u8], value: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let k = HostBuf::bytes(key);
        let v = HostBuf::bytes(value);
        abi::assembly_script_set_data(k.ptr(), v.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.set(key, value))
}

pub fn set_of(address: &str, key: &[u8], value: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let k = HostBuf::bytes(key);
        let v = HostBuf::bytes(value);
        abi::assembly_script_set_data_for(a.ptr(), k.ptr(), v.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.set_of(address, key, value))
}

pub fn get(key: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_bytes(abi::assembly_script_get_data(HostBuf::bytes(key).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get(key))
}

pub fn get_of(address: &str, key: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let k = HostBuf::bytes(key);
        take_bytes(abi::assembly_script_get_data_for(a.ptr(), k.ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_of(address, key))
}

pub fn del(key: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_delete_data(HostBuf::bytes(key).ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.del(key))
}

pub fn delete_of(address: &str, key: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let k = HostBuf::bytes(key);
        abi::assembly_script_delete_data_for(a.ptr(), k.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.delete_of(address, key))
}

pub fn append(key: &[u8], value: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let k = HostBuf::bytes(key);
        let v = HostBuf::bytes(value);
        abi::assembly_script_append_data(k.ptr(), v.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.append(key, value))
}

pub fn append_of(address: &str, key: &[u8], value: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let k = HostBuf::bytes(key);
        let v = HostBuf::bytes(value);
        abi::assembly_script_append_data_for(a.ptr(), k.ptr(), v.ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.append_of(address, key, value))
}

pub fn has(key: &[u8]) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_has_data(HostBuf::bytes(key).ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.has(key))
}

pub fn has_of(address: &str, key: &[u8]) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let k = HostBuf::bytes(key);
        abi::assembly_script_has_data_for(a.ptr(), k.ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.has_of(address, key))
}

/// All keys starting with `prefix` (empty prefix = all keys).
/// From MIP-0002 this fails when more than
/// [`crate::constants::MAX_DATASTORE_KEYS_PAGE`] keys match: prefer
/// [`get_keys_page`] on a datastore that can grow.
pub fn get_keys(prefix: &[u8]) -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        decode_keys(&take_bytes(abi::assembly_script_get_keys(HostBuf::bytes(prefix).ptr())))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_keys(prefix))
}

pub fn get_keys_of(address: &str, prefix: &[u8]) -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let p = HostBuf::bytes(prefix);
        decode_keys(&take_bytes(abi::assembly_script_get_keys_for(a.ptr(), p.ptr())))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_keys_of(address, prefix))
}

/// One page of keys (ascending byte order), strictly after `start_key`
/// (empty = from the beginning). Requires execution version >= 2 (MIP-0002).
pub fn get_keys_page(prefix: &[u8], start_key: &[u8], count: u32) -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let p = HostBuf::bytes(prefix);
        let s = HostBuf::bytes(start_key);
        decode_keys(&take_bytes(abi::assembly_script_get_keys_paginated(p.ptr(), s.ptr(), count as i32)))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_keys_page(prefix, start_key, count))
}

/// See [`get_keys_page`].
pub fn get_keys_of_page(address: &str, prefix: &[u8], start_key: &[u8], count: u32) -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(address);
        let p = HostBuf::bytes(prefix);
        let s = HostBuf::bytes(start_key);
        decode_keys(&take_bytes(abi::assembly_script_get_keys_for_paginated(
            a.ptr(),
            p.ptr(),
            s.ptr(),
            count as i32,
        )))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_keys_of_page(address, prefix, start_key, count))
}

// ---------------------------------------------------------------------------
// context
// ---------------------------------------------------------------------------

pub fn owned_addresses() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        decode_json_strings(&take_string(abi::assembly_script_get_owned_addresses()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.owned_addresses())
}

pub fn call_stack() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        decode_json_strings(&take_string(abi::assembly_script_get_call_stack()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.call_stack())
}

/// `n`-th address of the call stack counted from the top (`0` = current
/// contract, `1` = its caller...). Cheaper than [`call_stack`] when a single
/// entry is needed.
pub fn call_stack_from_top(n: usize) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        crate::codec::json_string_array_from_end(&take_string(abi::assembly_script_get_call_stack()), n)
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| {
        let stack = vm.call_stack();
        stack.len().checked_sub(n + 1).map(|i| stack[i].clone())
    })
}

pub fn call_coins() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_call_coins() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.call_coins())
}

/// Slot timestamp, milliseconds since the Unix epoch.
pub fn time() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_time() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.time())
}

pub fn current_period() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_current_period() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.current_period())
}

pub fn current_thread() -> u8 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_current_thread() as u8
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.current_thread())
}

/// Empty string when the execution was not started by an operation.
pub fn origin_operation_id() -> String {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_string(abi::assembly_script_get_origin_operation_id())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.origin_operation_id())
}

pub fn chain_id() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_chain_id() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.chain_id())
}

/// Predictable pseudo-random number: never use it for anything valuable.
pub fn unsafe_random() -> i64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_unsafe_random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.unsafe_random())
}

// ---------------------------------------------------------------------------
// coins
// ---------------------------------------------------------------------------

pub fn transfer_coins(to: &str, amount: u64) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_transfer_coins(HostBuf::string(to).ptr(), amount as i64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.transfer_coins(to, amount))
}

pub fn transfer_coins_of(from: &str, to: &str, amount: u64) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let f = HostBuf::string(from);
        let t = HostBuf::string(to);
        abi::assembly_script_transfer_coins_for(f.ptr(), t.ptr(), amount as i64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.transfer_coins_of(from, to, amount))
}

pub fn balance() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_balance() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.balance())
}

pub fn balance_of(address: &str) -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_balance_for(HostBuf::string(address).ptr()) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.balance_of(address))
}

// ---------------------------------------------------------------------------
// crypto / addresses
// ---------------------------------------------------------------------------

pub fn blake3(data: &[u8]) -> [u8; 32] {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_hash(abi::assembly_script_hash(HostBuf::bytes(data).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.blake3(data))
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_hash(abi::assembly_script_hash_sha256(HostBuf::bytes(data).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.sha256(data))
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_hash(abi::assembly_script_keccak256_hash(HostBuf::bytes(data).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.keccak256(data))
}

/// C-Layer only: MiMC hash over BN254 (`assembly_script_hash_mimc`).
pub fn mimc(data: &[u8]) -> [u8; 32] {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_hash(abi::assembly_script_hash_mimc(HostBuf::bytes(data).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.mimc(data))
}

/// Massa signature check. `signature` and `public_key` are base58check strings.
pub fn is_signature_valid(data: &str, signature: &str, public_key: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let d = HostBuf::string(data);
        let s = HostBuf::string(signature);
        let p = HostBuf::string(public_key);
        abi::assembly_script_signature_verify(d.ptr(), s.ptr(), p.ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.is_signature_valid(data, signature, public_key))
}

pub fn is_evm_signature_valid(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let d = HostBuf::bytes(data);
        let s = HostBuf::bytes(signature);
        let p = HostBuf::bytes(public_key);
        abi::assembly_script_evm_signature_verify(d.ptr(), s.ptr(), p.ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.is_evm_signature_valid(data, signature, public_key))
}

pub fn evm_get_address_from_pubkey(public_key: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_bytes(abi::assembly_script_evm_get_address_from_pubkey(HostBuf::bytes(public_key).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.evm_get_address_from_pubkey(public_key))
}

pub fn evm_get_pubkey_from_signature(hash: &[u8], signature: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let h = HostBuf::bytes(hash);
        let s = HostBuf::bytes(signature);
        take_bytes(abi::assembly_script_evm_get_pubkey_from_signature(h.ptr(), s.ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.evm_get_pubkey_from_signature(hash, signature))
}

pub fn is_address_eoa(address: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_is_address_eoa(HostBuf::string(address).ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.is_address_eoa(address))
}

pub fn public_key_to_address(public_key: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_string(abi::assembly_script_address_from_public_key(HostBuf::string(public_key).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.public_key_to_address(public_key))
}

pub fn validate_address(address: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_validate_address(HostBuf::string(address).ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.validate_address(address))
}

// ---------------------------------------------------------------------------
// operation datastore
// ---------------------------------------------------------------------------

pub fn get_op_keys() -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        decode_keys(&take_bytes(abi::assembly_script_get_op_keys()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_op_keys(&[]))
}

pub fn get_op_keys_prefix(prefix: &[u8]) -> Vec<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        decode_keys(&take_bytes(abi::assembly_script_get_op_keys_prefix(HostBuf::bytes(prefix).ptr())))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_op_keys(prefix))
}

pub fn has_op_key(key: &[u8]) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        // The host answers with a one-byte buffer.
        take_bytes(abi::assembly_script_has_op_key(HostBuf::bytes(key).ptr())).first() == Some(&1)
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.has_op_key(key))
}

pub fn get_op_data(key: &[u8]) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        take_bytes(abi::assembly_script_get_op_data(HostBuf::bytes(key).ptr()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.get_op_data(key))
}

// ---------------------------------------------------------------------------
// asynchronous messages & deferred calls
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn send_message(
    target_address: &str,
    target_function: &str,
    validity_start: (u64, u8),
    validity_end: (u64, u8),
    max_gas: u64,
    raw_fee: u64,
    coins: u64,
    params: &[u8],
    filter_address: &str,
    filter_key: &[u8],
) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(target_address);
        let f = HostBuf::string(target_function);
        let p = HostBuf::bytes(params);
        let fa = HostBuf::string(filter_address);
        let fk = HostBuf::bytes(filter_key);
        abi::assembly_script_send_message(
            a.ptr(),
            f.ptr(),
            validity_start.0 as i64,
            validity_start.1 as i32,
            validity_end.0 as i64,
            validity_end.1 as i32,
            max_gas as i64,
            raw_fee as i64,
            coins as i64,
            p.ptr(),
            fa.ptr(),
            fk.ptr(),
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| {
        vm.send_message(
            target_address,
            target_function,
            validity_start,
            validity_end,
            max_gas,
            raw_fee,
            coins,
            params,
            filter_address,
            filter_key,
        )
    })
}

/// Price (nanoMAS) of booking a deferred call in `slot`; 0 when the slot is full.
pub fn deferred_call_quote(slot: (u64, u8), max_gas: u64, params_size: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_get_deferred_call_quote(
            slot.0 as i64,
            slot.1 as i32,
            max_gas as i64,
            params_size as i64,
        ) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.deferred_call_quote(slot, max_gas, params_size))
}

/// Book a deferred call; returns its id.
pub fn deferred_call_register(
    target_address: &str,
    target_function: &str,
    slot: (u64, u8),
    max_gas: u64,
    params: &[u8],
    coins: u64,
) -> String {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let a = HostBuf::string(target_address);
        let f = HostBuf::string(target_function);
        let p = HostBuf::bytes(params);
        take_string(abi::assembly_script_deferred_call_register(
            a.ptr(),
            f.ptr(),
            slot.0 as i64,
            slot.1 as i32,
            max_gas as i64,
            p.ptr(),
            coins as i64,
        ))
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.deferred_call_register(target_address, target_function, slot, max_gas, params, coins))
}

pub fn deferred_call_exists(id: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_deferred_call_exists(HostBuf::string(id).ptr()) != 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.deferred_call_exists(id))
}

pub fn deferred_call_cancel(id: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        abi::assembly_script_deferred_call_cancel(HostBuf::string(id).ptr())
    }
    #[cfg(not(target_arch = "wasm32"))]
    with_vm(|vm| vm.deferred_call_cancel(id))
}
