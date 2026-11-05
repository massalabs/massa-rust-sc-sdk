# Interoperability with AssemblyScript contracts

A Rust contract built with this SDK is, for the Massa runtime and for every
other contract or client, indistinguishable from an AssemblyScript contract
built with `@massalabs/massa-as-sdk`. This page lists what that means
precisely and how it is verified.

## The contract model is the same

| | AssemblyScript | Rust |
|---|---|---|
| module format | wasm module importing `massa.*` / `env.abort`, exporting `__new`/`__pin` and `memory` | same (see [memory.md](memory.md)) |
| entry point | `export function f(args: StaticArray<u8>): StaticArray<u8>` (or `void`, or no parameter) | `#[export] pub fn f(args: Args) -> T` with `T: IntoReturn` (or `()`, or no parameter) |
| parameter | one serialized buffer, decoded with `Args` | one serialized buffer, decoded with `Args` |
| return value | raw bytes (`stringToBytes(s)`, `u64ToBytes(n)`, `boolToByte(b)`, `u256ToBytes(x)`, `args.serialize()`) | `String`, `u64`, `bool`, `U256`, `Args`, `Vec<u8>`... encode to the same raw bytes |
| deployment | `constructor(args)` called once by the deployer with write access; `isDeployingContract()` guard | `constructor(args)` + `context::is_deploying_contract()`; same deployer (`massa-web3`, station, the `deployer` example) |
| `ExecuteSC` | `main(args)` | `main()` / `main(args)` |
| aborting | `assert`, `throw`, `ERROR` | `assert!`, `panic!`, `abort()`, `Result::Err` from an entry point |
| events | `generateEvent(s)`, `createEvent(key, args)`, `generateRawEvent(bytes)` | `generate_event`, `create_event`, `generate_raw_event` (same `key:a,b` and base64 formats) |

Function names are not constrained by the protocol; the Massa ecosystem uses
camelCase (`balanceOf`). `#[export]` keeps the Rust name unless you pass
`#[export(name = "balanceOf")]`; the standards macros export the canonical
camelCase names.

## `Args` is byte-identical to `@massalabs/as-types`

| value | bytes | AS | Rust |
|---|---|---|---|
| `bool` | 1 | `add(b)` / `nextBool()` | `add(b)` / `next_bool()` |
| `u8`..`u64`, `i8`..`i64`, `f32`, `f64` | natural width, little-endian | `add<u64>(n)` / `nextU64()` | `add(n)` / `next_u64()` |
| `u128`, `i128`, `u256` | 16 / 16 / 32 LE | `add(u256)` / `nextU256()` | `add(U256)` / `next_u256()` |
| string | `u32` length + UTF-8 | `add(s)` / `nextString()` | `add("s")` / `next_string()` |
| bytes | `u32` length + bytes | `add(StaticArray<u8>)` / `nextBytes()` | `add(vec)` / `next_bytes()` |
| array of numbers | `u32` byte length + packed elements | `add<Array<u64>>(a)` / `nextFixedSizeArray<u64>()` | `add(vec![1u64])` / `next_array::<u64>()` |
| array of strings | `u32` byte length + (`u32` length + UTF-8)* | `add<Array<string>>(a)` / `nextStringArray()` | `add(vec!["a"])` / `next_string_array()` |
| structure | fields concatenated, no prefix | `class X implements Serializable` / `nextSerializable<X>()` | `#[derive(Serializable)]` / `next::<X>()` |
| array of structures | `u32` byte length + structures | `addSerializableObjectArray` / `nextSerializableObjectArray` | `add(vec![x])` / `next_array::<X>()` |
| `Address` | as a string | `add(address)` / `nextSerializable<Address>()` | `add(&address)` / `next_address()` |
| nested `Args` | as bytes | `add(inner.serialize())` / `new Args(nextBytes())` | `add(inner.serialize())` / `Args::from(next_bytes()?)` |

`Option<T>` (Rust only) is a `bool` followed by the value: what an
AssemblyScript side writes as `add(present).add(value)`.

Calls between contracts send exactly these bytes (`call(addr, "f", args,
coins)` on both sides), so a Rust contract calls an AssemblyScript one, and
the reverse, without any adapter; the `Mrc20` client of `massa-sc-standards`
and the `MRC20Wrapper` of `@massalabs/sc-standards` drive each other's tokens.

## Storage layouts are the same

| helper | AS | Rust | layout |
|---|---|---|---|
| raw datastore | `Storage.set(key, value)` | `storage::set(key, value)` | bytes as given |
| typed values | `u64ToBytes`, `stringToBytes`, `boolToByte`... | `storage::set_value(key, &v)` (`StorageValue`) | natural width LE, raw UTF-8, 1 byte |
| `PersistentMap<K, V>` | `collections.PersistentMap` | `collections::PersistentMap` | key `"<prefix>::<key>"`; integers, floats **and booleans** on 8 bytes; strings/bytes raw; structures serialized |
| `ConstantManager<T>` | same name | `helpers::ConstantManager` | key from `KeyIncrementer` (`Args(counter)`), value `Args(v)` |
| `MapManager<K, V>` | same name | `helpers::MapManager` | key `prefix ++ Args(k)`, value `Args(v)` |
| `AccessControl<T>` (bitmask) | `security.AccessControl` | `security::AccessControl` | key `[moduleId] ++ Args(address)`, value `T` LE |

`massa-sc-standards` reproduces `@massalabs/sc-standards` key by key: MRC20
(`NAME`, `SYMBOL`, `DECIMALS`, `TOTAL_SUPPLY`, `BALANCE<addr>`,
`ALLOWANCE<owner><spender>`, `OWNER`), MRC721 (`0x01`..`0x06` prefixes,
`BASE_URI`, `TOKENS_URI<id>`, `totalSupply`, `ownedTokens<owner><id>`),
MRC1155 (`0x01`..`0x03` prefixes, URIs, `_ROLES<role>`), with the same events
and entry-point names. The bytecode of one implementation can replace the
other on top of its datastore.

## Verification: the interoperability suite

[`interop/`](../interop) compiles AssemblyScript fixtures with `asc` from the
real `@massalabs/massa-as-sdk` and `@massalabs/sc-standards`, builds their
Rust counterparts with `cargo massa build`, and
[`tools/massa-sc-runner/tests/interop.rs`](../tools/massa-sc-runner/tests/interop.rs)
executes them together in the real Massa runtime:

* `Args`: every type above, encoded by one side and decoded by the other,
  re-encoded and compared byte for byte; golden vectors from both sides;
* return idioms (`retString`, `retU64`, `retBool`, `retU256`, `retArgs`,
  `void`, no parameter) produce identical bytes;
* cross calls in every combination (Rust caller → AS token, AS caller → Rust
  token, and the two homogeneous ones): metadata, balances, transfers,
  allowance flows, storage-cost queries, generic forwarding, `functionExists`,
  error propagation, call stack (`caller`), coins travelling with calls,
  `localCall`;
* MRC20, MRC721, MRC1155 (+ ownership and roles): the same operation sequence
  on both implementations yields byte-identical datastores, events and return
  values, and each implementation keeps working on the datastore produced by
  the other;
* storage helpers: each SDK writes with `Storage`, `PersistentMap`,
  `ConstantManager`, `MapManager` and `AccessControl`, each SDK reads it back,
  in every combination, with identical datastores.

```sh
SC_STANDARDS=path/to/massa-standards/smart-contracts interop/build.sh   # or without SC_STANDARDS: the published package
cd tools/massa-sc-runner && cargo test --release --test interop
```

CI runs it against `massa-standards` main (strict) and against the published
`@massalabs/sc-standards` 1.3.0, whose only difference is that it still
stores an exhausted balance as 32 zero bytes where main and Rust delete the
entry (both read zero; `INTEROP_LEGACY_STANDARDS=1` tolerates it).

## Networks

* The SDK only uses the AssemblyScript-module ABI; the protobuf `wasmv1` ABI
  (disabled by MIP-0002) is not used anywhere, nothing depends on protobuf.
* C-Layer nodes have the same ABI plus `mimc`, and do not expose the MIP-0002
  paginated-key imports yet. Nothing in the SDK or standards imports them
  unless you call `storage::get_keys_page` / `for_each_key` /
  `owned_tokens_page` explicitly. `cargo massa build --network clayer|massa|any`
  fails the build on imports the target network lacks.
