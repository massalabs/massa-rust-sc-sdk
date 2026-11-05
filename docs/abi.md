# Host ABI reference

Every host function imported by a Massa module, the Rust wrapper that calls
it, and its gas cost (`massa-node/base_config/gas_costs/abi_gas_costs.json`,
on top of 23 gas per executed wasm instruction). Imports only appear in the
compiled module when the contract uses them, so unused ABIs cost nothing and
C-Layer-only ABIs do not prevent deployment on Massa unless called.

Pointers are `i32` offsets in linear memory; `buf` is a byte buffer and
`str` a UTF-16LE string, both with their byte length stored in the 4 bytes
before the payload (see [memory.md](memory.md)).

## `massa` module

| host import | signature | `massa_sc_sdk::env` | higher level | gas |
|---|---|---|---|---|
| `assembly_script_print` | `(str)` | `print` | node logs only, no on-chain effect | 855 |
| `assembly_script_generate_event` | `(str)` | `generate_event` | `generate_event`, `generate_raw_event` | 860 |
| `assembly_script_call` | `(str addr, str fn, buf args, i64 coins) -> buf` | `call` | `contract::call` | 15000 |
| `assembly_script_local_call` | `(str, str, buf) -> buf` | `local_call` | `contract::local_call` | 15000 |
| `assembly_script_local_execution` | `(buf bytecode, str, buf) -> buf` | `local_execution` | `contract::local_execution` | 314000000 |
| `assembly_script_create_sc` | `(buf) -> str` | `create_sc` | `contract::create_sc` | 745000000 |
| `assembly_script_get_bytecode` | `() -> buf` | `get_bytecode` | `contract::get_bytecode` | 1100 |
| `assembly_script_get_bytecode_for` | `(str) -> buf` | `get_bytecode_of` | `contract::get_bytecode_of` | 1375 |
| `assembly_script_set_bytecode` | `(buf)` | `set_bytecode` | `contract::set_bytecode` | 745000000 |
| `assembly_script_set_bytecode_for` | `(str, buf)` | `set_bytecode_of` | `contract::set_bytecode_of` | 745000000 |
| `assembly_script_function_exists` | `(str, str) -> i32` | `function_exists` | `contract::function_exists` | 2875 |
| `assembly_script_caller_has_write_access` | `() -> i32` | `caller_has_write_access` | `contract::caller_has_write_access`, `context::is_deploying_contract` | 775 |
| `assembly_script_get_remaining_gas` | `() -> i64` | `remaining_gas` | `context::remaining_gas` | 580 |
| `assembly_script_set_data` | `(buf key, buf value)` | `set` | `storage::set`, `set_value` | 940 |
| `assembly_script_set_data_for` | `(str, buf, buf)` | `set_of` | `storage::set_of` | 1070 |
| `assembly_script_get_data` | `(buf) -> buf` | `get` | `storage::get`, `try_get`, `get_value` | 1040 |
| `assembly_script_get_data_for` | `(str, buf) -> buf` | `get_of` | `storage::get_of` | 1240 |
| `assembly_script_delete_data` | `(buf)` | `del` | `storage::del`, `del_if_exists` | 980 |
| `assembly_script_delete_data_for` | `(str, buf)` | `delete_of` | `storage::del_of` | 1100 |
| `assembly_script_append_data` | `(buf, buf)` | `append` | `storage::append` | 1060 |
| `assembly_script_append_data_for` | `(str, buf, buf)` | `append_of` | `storage::append_of` | 1290 |
| `assembly_script_has_data` | `(buf) -> i32` | `has` | `storage::has` | 845 |
| `assembly_script_has_data_for` | `(str, buf) -> i32` | `has_of` | `storage::has_of` | 1220 |
| `assembly_script_get_keys` | `(buf prefix) -> buf` | `get_keys` | `storage::get_keys` (fails above 500 keys since MIP-0002) | 1000 |
| `assembly_script_get_keys_for` | `(str, buf) -> buf` | `get_keys_of` | `storage::get_keys_of` | 1195 |
| `assembly_script_get_keys_paginated` | `(buf prefix, buf start, i32 count) -> buf` | `get_keys_page` | `storage::get_keys_page`, `for_each_key` (MIP-0002) | 190000 |
| `assembly_script_get_keys_for_paginated` | `(str, buf, buf, i32) -> buf` | `get_keys_of_page` | `storage::get_keys_of_page` (MIP-0002) | 190000 |
| `assembly_script_get_owned_addresses` | `() -> str (JSON)` | `owned_addresses` | `context::owned_addresses` | 1600 |
| `assembly_script_get_call_stack` | `() -> str (JSON)` | `call_stack`, `call_stack_from_top` | `context::call_stack/caller/callee/transaction_creator` | 1560 |
| `assembly_script_get_call_coins` | `() -> i64` | `call_coins` | `context::transferred_coins` | 725 |
| `assembly_script_get_time` | `() -> i64` | `time` | `context::timestamp` | 750 |
| `assembly_script_get_current_period` | `() -> i64` | `current_period` | `context::current_period`, `Slot::current` | 785 |
| `assembly_script_get_current_thread` | `() -> i32` | `current_thread` | `context::current_thread` | 770 |
| `assembly_script_get_origin_operation_id` | `() -> str` | `origin_operation_id` | `context::origin_operation_id` | 785 |
| `assembly_script_chain_id` | `() -> i64` | `chain_id` | `context::chain_id`, `is_clayer` | 785 |
| `assembly_script_unsafe_random` | `() -> i64` | `unsafe_random` | `crypto::unsafe_random` | 790 |
| `assembly_script_transfer_coins` | `(str, i64)` | `transfer_coins` | `coins::transfer_coins`, `transfer_remaining` | 1045 |
| `assembly_script_transfer_coins_for` | `(str, str, i64)` | `transfer_coins_of` | `coins::transfer_coins_of` | 1190 |
| `assembly_script_get_balance` | `() -> i64` | `balance` | `coins::balance` | 745 |
| `assembly_script_get_balance_for` | `(str) -> i64` | `balance_of` | `coins::balance_of`, `Address::balance` | 900 |
| `assembly_script_hash` | `(buf) -> buf` | `blake3` | `crypto::blake3` | 1055 |
| `assembly_script_hash_sha256` | `(buf) -> buf` | `sha256` | `crypto::sha256` | 990 |
| `assembly_script_keccak256_hash` | `(buf) -> buf` | `keccak256` | `crypto::keccak256` | 1055 |
| `assembly_script_hash_mimc` | `(buf) -> buf` | `mimc` | `clayer::mimc` (**C-Layer only**) | n/a |
| `assembly_script_signature_verify` | `(str data, str sig, str pubkey) -> i32` | `is_signature_valid` | `crypto::is_signature_valid` | 1200 |
| `assembly_script_evm_signature_verify` | `(buf, buf, buf) -> i32` | `is_evm_signature_valid` | `crypto::is_evm_signature_valid` | 3310 |
| `assembly_script_evm_get_address_from_pubkey` | `(buf) -> buf` | `evm_get_address_from_pubkey` | `crypto::...` | 1120 |
| `assembly_script_evm_get_pubkey_from_signature` | `(buf, buf) -> buf` | `evm_get_pubkey_from_signature` | `crypto::...` | 1540 |
| `assembly_script_is_address_eoa` | `(str) -> i32` | `is_address_eoa` | `crypto::is_address_eoa`, `Address::is_eoa` | 450 |
| `assembly_script_address_from_public_key` | `(str) -> str` | `public_key_to_address` | `crypto::public_key_to_address` | 1570 |
| `assembly_script_validate_address` | `(str) -> i32` | `validate_address` | `Address::is_valid` | 890 |
| `assembly_script_get_op_keys` | `() -> buf` | `get_op_keys` | `op_datastore::get_op_keys` | 1400 |
| `assembly_script_get_op_keys_prefix` | `(buf) -> buf` | `get_op_keys_prefix` | `op_datastore::get_op_keys_prefix` | 1400 |
| `assembly_script_has_op_key` | `(buf) -> buf(1)` | `has_op_key` | `op_datastore::has_op_key` | 1455 |
| `assembly_script_get_op_data` | `(buf) -> buf` | `get_op_data` | `op_datastore::get_op_data` | 50000 |
| `assembly_script_send_message` | `(str, str, i64, i32, i64, i32, i64, i64, i64, buf, str, buf)` | `send_message` | `contract::async_call` | 40000000 |
| `assembly_script_get_deferred_call_quote` | `(i64, i32, i64, i64) -> i64` | `deferred_call_quote` | `deferred_calls::quote` | 1220 |
| `assembly_script_deferred_call_register` | `(str, str, i64, i32, i64, buf, i64) -> str` | `deferred_call_register` | `deferred_calls::register` | 2650 |
| `assembly_script_deferred_call_exists` | `(str) -> i32` | `deferred_call_exists` | `deferred_calls::exists` | 6580 |
| `assembly_script_deferred_call_cancel` | `(str)` | `deferred_call_cancel` | `deferred_calls::cancel` | 4165 |

## `env` module (AssemblyScript runtime)

| host import | used by |
|---|---|
| `abort(str message, str file, i32 line, i32 col)` | the SDK panic handler: a Rust `panic!`/`assert!` aborts the execution with `error: <message> at <file>:<line> col: <col>` |

`seed`, `Date.now`, `console.*`, `trace`, `process.exit` are also exported by
the host for AssemblyScript but have no Rust counterpart.

## Serialized formats

* **Key lists** (`get_keys*`, `get_op_keys*`): `count: u32 LE`, then `count`
  times `len: u8` + key bytes. Empty buffer = no key. Decoded by
  `env::decode_keys`.
* **Address lists** (`call_stack`, `owned_addresses`): JSON array of strings.
* **Booleans from `has_op_key`**: a 1-byte buffer.
* **Hashes**: 32-byte buffers.

## Execution limits that matter

* 4 MiB of linear memory per execution (64 pages).
* At most 768 functions (imports included), 257 imports, 1024 exports,
  1 custom section, 16 tables, 512 globals / data segments / element segments.
* Bytecode at most 10 MB; each byte costs 0.0001 MAS of storage at deployment.
* Events are capped at 1 KiB; datastore keys at 255 bytes.
* Entry points are called as `(i32) -> i32`, `() -> i32`, `(i32) -> ()` or
  `() -> ()`; `main` is the entry point of `ExecuteSC` operations and its
  return value is ignored.
