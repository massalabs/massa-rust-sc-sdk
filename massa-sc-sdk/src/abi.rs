//! Raw host imports (the "ABI"), exactly as declared by `massa-sc-runtime`
//! (`src/as_execution/context.rs`). Pointers are `i32` offsets into linear
//! memory following the layout described in [`crate::memory`]; amounts, gas
//! and timestamps are `i64`/`u64`.
//!
//! Only imports that are actually referenced end up in the compiled module,
//! so a contract that never calls C-Layer-only ABIs (e.g. `mimc`) links
//! fine on Massa, and vice versa.
#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
pub(crate) mod env {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn abort(message: i32, filename: i32, line: i32, col: i32);
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod massa {
    #[link(wasm_import_module = "massa")]
    extern "C" {
        // --- logging / events ---
        pub fn assembly_script_print(message: i32);
        pub fn assembly_script_generate_event(event: i32);

        // --- calls & bytecode ---
        pub fn assembly_script_call(address: i32, function: i32, param: i32, coins: i64) -> i32;
        pub fn assembly_script_local_call(address: i32, function: i32, param: i32) -> i32;
        pub fn assembly_script_local_execution(bytecode: i32, function: i32, param: i32) -> i32;
        pub fn assembly_script_create_sc(bytecode: i32) -> i32;
        pub fn assembly_script_get_bytecode() -> i32;
        pub fn assembly_script_get_bytecode_for(address: i32) -> i32;
        pub fn assembly_script_set_bytecode(bytecode: i32);
        pub fn assembly_script_set_bytecode_for(address: i32, bytecode: i32);
        pub fn assembly_script_function_exists(address: i32, function: i32) -> i32;
        pub fn assembly_script_caller_has_write_access() -> i32;
        pub fn assembly_script_get_remaining_gas() -> i64;

        // --- datastore ---
        pub fn assembly_script_set_data(key: i32, value: i32);
        pub fn assembly_script_set_data_for(address: i32, key: i32, value: i32);
        pub fn assembly_script_get_data(key: i32) -> i32;
        pub fn assembly_script_get_data_for(address: i32, key: i32) -> i32;
        pub fn assembly_script_delete_data(key: i32);
        pub fn assembly_script_delete_data_for(address: i32, key: i32);
        pub fn assembly_script_append_data(key: i32, value: i32);
        pub fn assembly_script_append_data_for(address: i32, key: i32, value: i32);
        pub fn assembly_script_has_data(key: i32) -> i32;
        pub fn assembly_script_has_data_for(address: i32, key: i32) -> i32;
        pub fn assembly_script_get_keys(prefix: i32) -> i32;
        pub fn assembly_script_get_keys_for(address: i32, prefix: i32) -> i32;
        /// Available from execution version 2 (MIP-0002).
        pub fn assembly_script_get_keys_paginated(prefix: i32, start_key: i32, count: i32) -> i32;
        /// Available from execution version 2 (MIP-0002).
        pub fn assembly_script_get_keys_for_paginated(
            address: i32,
            prefix: i32,
            start_key: i32,
            count: i32,
        ) -> i32;

        // --- context ---
        pub fn assembly_script_get_owned_addresses() -> i32;
        pub fn assembly_script_get_call_stack() -> i32;
        pub fn assembly_script_get_call_coins() -> i64;
        pub fn assembly_script_get_time() -> i64;
        pub fn assembly_script_get_current_period() -> i64;
        pub fn assembly_script_get_current_thread() -> i32;
        pub fn assembly_script_get_origin_operation_id() -> i32;
        pub fn assembly_script_chain_id() -> i64;
        pub fn assembly_script_unsafe_random() -> i64;

        // --- coins ---
        pub fn assembly_script_transfer_coins(to_address: i32, raw_amount: i64);
        pub fn assembly_script_transfer_coins_for(from_address: i32, to_address: i32, raw_amount: i64);
        pub fn assembly_script_get_balance() -> i64;
        pub fn assembly_script_get_balance_for(address: i32) -> i64;

        // --- crypto / addresses ---
        pub fn assembly_script_hash(data: i32) -> i32;
        pub fn assembly_script_hash_sha256(data: i32) -> i32;
        pub fn assembly_script_keccak256_hash(data: i32) -> i32;
        /// C-Layer only.
        pub fn assembly_script_hash_mimc(data: i32) -> i32;
        pub fn assembly_script_signature_verify(data: i32, signature: i32, public_key: i32) -> i32;
        pub fn assembly_script_evm_signature_verify(data: i32, signature: i32, public_key: i32) -> i32;
        pub fn assembly_script_evm_get_address_from_pubkey(public_key: i32) -> i32;
        pub fn assembly_script_evm_get_pubkey_from_signature(hash: i32, signature: i32) -> i32;
        pub fn assembly_script_is_address_eoa(address: i32) -> i32;
        pub fn assembly_script_address_from_public_key(public_key: i32) -> i32;
        pub fn assembly_script_validate_address(address: i32) -> i32;

        // --- operation datastore ---
        pub fn assembly_script_get_op_keys() -> i32;
        pub fn assembly_script_get_op_keys_prefix(prefix: i32) -> i32;
        pub fn assembly_script_has_op_key(key: i32) -> i32;
        pub fn assembly_script_get_op_data(key: i32) -> i32;

        // --- asynchronous messages & deferred calls ---
        #[allow(clippy::too_many_arguments)]
        pub fn assembly_script_send_message(
            target_address: i32,
            target_handler: i32,
            validity_start_period: i64,
            validity_start_thread: i32,
            validity_end_period: i64,
            validity_end_thread: i32,
            max_gas: i64,
            raw_fee: i64,
            raw_coins: i64,
            data: i32,
            filter_address: i32,
            filter_datastore_key: i32,
        );
        pub fn assembly_script_get_deferred_call_quote(
            period: i64,
            thread: i32,
            max_gas: i64,
            params_size: i64,
        ) -> i64;
        pub fn assembly_script_deferred_call_register(
            target_address: i32,
            target_function: i32,
            target_period: i64,
            target_thread: i32,
            max_gas: i64,
            params: i32,
            raw_coins: i64,
        ) -> i32;
        pub fn assembly_script_deferred_call_exists(id: i32) -> i32;
        pub fn assembly_script_deferred_call_cancel(id: i32);
    }
}
