# Migrating from the AssemblyScript SDK

The Rust SDK mirrors `@massalabs/massa-as-sdk` namespace by namespace, with
Rust naming (`snake_case` functions, modules instead of namespaces).
Serialized data (`Args`), event formats and the standards' storage layouts
are byte-identical, so a Rust contract can replace an AssemblyScript one
behind the same `massa-web3` client code.

## Entry points

```ts
export function transfer(binaryArgs: StaticArray<u8>): void {
  const args = new Args(binaryArgs);
  const to = new Address(args.nextString().expect('to is missing'));
  const amount = args.nextU64().expect('amount is missing');
  ...
}
```

```rust
#[export]
pub fn transfer(mut args: Args) {
    let to: Address = args.must_next("to");
    let amount: u64 = args.must_next("amount");
    ...
}
```

* Exported names default to the Rust name; use `#[export(name = "balanceOf")]`
  to keep camelCase names expected by clients.
* Return `Vec<u8>`, `Args`, `String`/`&str` (raw UTF-8), integers (little
  endian), `bool`, `U256`, `Address`, `Option<T>` (empty when `None`) or
  `Result<T, E>` (an `Err` aborts with its message). `()` returns nothing.
* `assert(cond, msg)` → `assert!(cond, "msg")`; `throw`/`ERROR` →
  `panic!`/`abort(msg)`. All of them abort the execution with the message in
  the operation events.

## Arguments

| as-types | Rust |
|---|---|
| `new Args().add(x).add(y).serialize()` | `Args::new().add(x).add(y)` (`args.serialize()` / `as_bytes()` for the bytes; `call` accepts the `Args` directly) |
| `add(inner.serialize())` (nested Args) | `add(inner.serialize())`, read back with `Args::from(args.next_bytes()?)` |
| `args.nextString().unwrap()` | `args.next_string()?` or `args.next::<String>()?` |
| `args.nextU256()` | `args.next_u256()` (`U256` from `ethnum`) |
| `args.nextSerializable<T>()` | `args.next::<T>()` with `#[derive(Serializable)]` |
| `args.nextSerializableObjectArray<T>()` | `args.next::<Vec<T>>()` |
| `args.nextFixedSizeArray<u64>()` | `args.next::<Vec<u64>>()` |
| `args.nextStringArray()` | `args.next::<Vec<String>>()` |
| `result.expect(msg)` | `args.must_next(msg)` or `.expect(msg)` |
| `stringToBytes`, `bytesToString` | `.as_bytes()`, `String::from_utf8` |
| `u64ToBytes`, `bytesToU64`, `u256ToBytes`... | `x.to_le_bytes()`, `u64::from_le_bytes`, `ArgsEncode::encode` / `ArgsDecode::decode_bytes` |

`Serializable` classes become structs:

```rust
#[derive(Serializable)]
struct Order { owner: Address, amount: u64, memo: String }
```

## Storage

| AssemblyScript | Rust |
|---|---|
| `Storage.set(key, value)` (string / bytes / Args) | `storage::set(key, value)` (any `AsRef<[u8]>`) |
| `Storage.get<StaticArray<u8>>(key)` | `storage::get(key)` (aborts when missing) |
| `Storage.has(key) ? Storage.get(key) : default` | `storage::try_get(key)`, `get_or(key, default)` |
| `Storage.set<string>(k, v)` / `get<string>` | `storage::set_value(k, &v)` / `get_value::<String>(k)` |
| `Storage.setOf(addr, k, v)` | `storage::set_of(&addr, k, v)` |
| `Storage.getKeys(prefix)` | `storage::get_keys(prefix)` (≤ 500 keys) |
| `Storage.getKeysPage(prefix, cursor, n)` | `storage::get_keys_page(prefix, cursor, n)`, `for_each_key` |
| `PersistentMap<K, V>` | `collections::PersistentMap<K, V>` (same keys and bytes; integers, floats and booleans on 8 bytes like AS) |
| `ConstantManager`, `MapManager`, `KeyIncrementer` | `helpers::*` |

## Context, coins, calls

| AssemblyScript | Rust |
|---|---|
| `Context.caller()`, `Context.callee()` | `context::caller()`, `context::callee()` |
| `Context.transferredCoins()` | `context::transferred_coins()` |
| `Context.timestamp()`, `currentPeriod()`, `currentThread()` | `context::timestamp()`, `current_period()`, `current_thread()`, `Slot::current()` |
| `isDeployingContract()` | `context::is_deploying_contract()` |
| `balance()`, `balanceOf(addr)` | `coins::balance()`, `coins::balance_of(&addr)` |
| `transferCoins(addr, amount)` | `coins::transfer_coins(&addr, amount)` |
| `transferRemaining(initialBalance)` | `coins::transfer_remaining(initial_balance, 0, None)` |
| `call(addr, 'fn', args, coins)` | `contract::call(&addr, "fn", &args, coins)` |
| `createSC(bytecode)` | `contract::create_sc(&bytecode)` |
| `fileToByteArray('./build/x.wasm')` | `include_bytes!("../build/x.wasm")` |
| `asyncCall(addr, fn, start, end, gas, fee, params, coins)` | `contract::async_call(&addr, "fn", start, end, gas, fee, coins, &params, None)` |
| `deferredCallRegister(...)`, `findCheapestSlot(...)` | `deferred_calls::register(...)`, `find_cheapest_slot(...) -> Option<Slot>` |
| `getOpData(key)`, `hasOpKey(key)` | `op_datastore::get_op_data(key)`, `has_op_key(key)` |
| `sha256`, `keccak256`, `blake3`, `isSignatureValid`, EVM helpers | `crypto::*` |
| `mimc(data)` (C-Layer) | `clayer::mimc(data)` (no chain-id assertion: the import itself makes the module C-Layer only) |

## Standards

`@massalabs/sc-standards` re-exports entry points from your contract file:

```ts
export { name, symbol, transfer, ... } from '@massalabs/sc-standards/assembly/contracts/MRC20';
```

In Rust the export macros generate them in your crate:

```rust
use massa_sc_standards::{export_mrc20, export_ownable, mrc20};
#[export] pub fn constructor(mut args: Args) { mrc20::constructor(&name, &symbol, decimals, supply); }
export_mrc20!();
export_ownable!();
```

Storage keys, event strings and entry-point names are the same, so
`massa-web3`'s `MRC20`, `MRC721`... helpers work unchanged.

## Tests

`as-pect` + `vm-mock` → `cargo test` + `massa_sc_sdk::mock`:

| vm-mock | Rust |
|---|---|
| `resetStorage()` | `mock::reset()` |
| `setDeployContext(caller?)` | `mock::set_deploy_context(Some(&caller))` |
| `mockAdminContext(true)` | `mock::set_admin_context(true)` |
| `mockTransferredCoins(n)` | `mock::set_transferred_coins(n)` |
| `mockBalance(addr, n)` | `mock::set_balance(&addr, n)` |
| `mockScCall(returnBytes)` | `mock::mock_any_call(bytes)` / `mock::mock_call(&addr, "fn", bytes)` / `mock::set_call_handler(...)` |
| `mockTimestamp`, `mockCurrentSlot`, `mockSetChainId`, `mockOriginOperationId` | `mock::set_time`, `set_slot`, `set_chain_id`, `set_origin_operation_id` |
| `setCallStack` | `mock::set_call_stack(&[...])`, `set_caller`, `set_callee` |
| reading events | `mock::events()` |

Tests run natively, in parallel, each with its own VM.

Everything in these tables is covered by the
[interoperability suite](interop.md), which runs AssemblyScript and Rust
contracts together in the real runtime.

## Build and deploy

`asc` + `asconfig.json` → `cargo massa build` (see [building.md](building.md)).
The deployment flow is unchanged: `massa-web3`'s `SmartContract.deploy` uses
the standard AssemblyScript deployer, which `createSC`s your module and calls
its `constructor`.
