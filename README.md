# Massa Rust smart-contract SDK

Write [Massa](https://massa.net) and [C-Layer](https://github.com/massalabs/c-layer)
smart contracts in Rust. The produced WebAssembly modules speak the same host
ABI as AssemblyScript contracts, so they deploy with the usual tooling
([massa-web3](https://github.com/massalabs/massa-web3), the station, explorers)
and interoperate with every existing contract, with the same `Args`
serialization and the same storage layouts for the standards.

| crate | what it is |
|-------|------------|
| [`massa-sc-sdk`](massa-sc-sdk) | the SDK: typed host ABI, `Args`, `Address`, storage/context/coins/calls/deferred calls helpers, collections, `#[export]` entry points, in-memory mock VM for `cargo test` |
| [`massa-sc-standards`](massa-sc-standards) | MRC20, MRC721, MRC1155, Ownable and role-based access control (ports of [massa-standards](https://github.com/massalabs/massa-standards)) |
| [`cargo-massa`](cargo-massa) | `cargo massa new / build / check`: scaffold, build with the right flags, optimize, validate against the node limits |
| [`tools/massa-sc-runner`](tools/massa-sc-runner) | run a `.wasm` locally inside the real Massa runtime (gas, events, state) without a node |

Examples (counter, tokens, NFT, cross-contract calls, deferred calls, deployer,
C-Layer MiMC...) with buildnet deployment scripts live in
[massa-rust-sc-examples](https://github.com/massalabs/massa-rust-sc-examples).

## Interoperable by construction

A Rust contract is, for the runtime, other contracts and clients, the same
thing as an AssemblyScript one: one serialized `Args` parameter per entry
point, the same `Args` bytes as `@massalabs/as-types` / `massa-web3`, the same
return idioms, `constructor` called by the same deployer, the same events,
and for the standards the same storage layouts and entry-point names as
`@massalabs/sc-standards`. Rust and AssemblyScript contracts call each other
transparently, and an MRC20/721/1155 written in one can be replaced by the
other on top of its datastore.

This is not a claim but a test: [`interop/`](interop) compiles AssemblyScript
fixtures with `asc` from the real AS SDK and standards, and
[`tools/massa-sc-runner/tests/interop.rs`](tools/massa-sc-runner/tests/interop.rs)
runs both sides together in the real Massa runtime (cross calls in every
direction, byte-for-byte `Args` round trips, identical datastores and events).
Details: [docs/interop.md](docs/interop.md).

## Quick start

```sh
rustup target add wasm32v1-none
cargo install --git https://github.com/massalabs/massa-rust-sc-sdk cargo-massa
cargo install wasm-opt           # optional: ~20% smaller modules, ~30% less gas

cargo massa new my-contract      # or: --template token
cd my-contract
cargo test                       # native unit tests on the mock VM
cargo massa build                # build/my_contract.wasm, ready to deploy
cd scripts && npm install && npm run deploy   # deploy + call it with massa-web3 (PRIVATE_KEY in .env)
```

A contract is a `cdylib` crate; entry points are plain functions marked
`#[export]`:

```rust
#![cfg_attr(target_arch = "wasm32", no_std)]
use massa_sc_sdk::prelude::*;

const COUNTER: &[u8] = b"counter";

#[export]
pub fn constructor(mut args: Args) {
    assert!(context::is_deploying_contract(), "constructor can only be called at deployment");
    let initial: u64 = args.next().unwrap_or(0);
    storage::set_value(COUNTER, &initial);
}

#[export]
pub fn increment(mut args: Args) -> u64 {
    let by: u64 = args.next().unwrap_or(1);
    let value = storage::try_get_value::<u64>(COUNTER).unwrap_or(0) + by;
    storage::set_value(COUNTER, &value);
    generate_event(&format!("Counter is now {value}"));
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn increments() {
        mock::reset();
        mock::set_deploy_context(None);
        constructor(Args::new().add(10u64));
        assert_eq!(increment(Args::new().add(5u64)), 15);
        assert_eq!(mock::events(), vec!["Counter is now 15"]);
    }
}
```

Deploy and call it from TypeScript exactly like an AssemblyScript contract:

```ts
const contract = await SmartContract.deploy(provider, bytecode, new Args().addU64(10n), { coins: Mas.fromString('0.1') });
await contract.call('increment', new Args().addU64(5n));
const value = bytesToU64((await contract.read('increment')).value);
```

## What is in the SDK

| AssemblyScript (`@massalabs/massa-as-sdk`) | Rust (`massa_sc_sdk::prelude::*`) |
|---|---|
| `Storage.set/get/has/del/append`, `*Of`, `getKeys`, `getKeysPage` | `storage::set/get/try_get/has/del/append`, `*_of`, `get_keys`, `get_keys_page`, `for_each_key`, typed `set_value/get_value` |
| `Context.caller/callee/callStack/transferredCoins/timestamp/...` | `context::caller/callee/call_stack/transferred_coins/timestamp/...`, `Slot` |
| `balance`, `transferCoins`, `transferRemaining` | `coins::balance/transfer_coins/transfer_remaining` |
| `call`, `localCall`, `createSC`, `functionExists`, `asyncCall`, bytecode | `contract::call/local_call/create_sc/function_exists/async_call/...` |
| `deferredCallRegister/Quote/Exists/Cancel`, `findCheapestSlot` | `deferred_calls::register/quote/exists/cancel/find_cheapest_slot` |
| `getOpData`, `hasOpKey`, `getOpKeys` | `op_datastore::*` |
| `sha256`, `keccak256`, `blake3`, signatures, EVM helpers, addresses | `crypto::*`, `Address` |
| `generateEvent`, `generateRawEvent`, `createEvent` | `generate_event`, `generate_raw_event`, `create_event` |
| `Args` (`@massalabs/as-types`), `Serializable`, `u256` | `Args`, `#[derive(Serializable)]`, `U256` |
| `PersistentMap`, `ConstantManager`, `MapManager`, `KeyIncrementer`, `AccessControl` | `collections::PersistentMap`, `helpers::*`, `security::AccessControl` |
| `mimc` (C-Layer) | `clayer::mimc` |
| vm-mock | `mock` (native targets): `set_deploy_context`, `set_caller`, `mock_call`, `events()`... |
| `env` (raw ABI) | `env` (typed, one function per host import) |

The ABI reference, memory protocol, size/gas guidance and a migration guide
are in [`docs/`](docs).

## Networks

Massa and [C-Layer](https://github.com/massalabs/c-layer) share the ABI;
C-Layer adds `clayer::mimc` and does not expose the MIP-0002 paginated-key
imports yet. The SDK only imports what a contract calls, so a contract runs
on both unless it uses one of those, and `cargo massa build --network
massa|clayer|any` fails the build when it does. Only the AssemblyScript-module
ABI is used: no protobuf, no `wasmv1`.

## Why `wasm32v1-none`

Massa's runtime (wasmer 4.3, deterministic feature set) only accepts MVP
WebAssembly plus bulk memory, sign extension and mutable globals. Rust's
`wasm32-unknown-unknown` target enables reference types and multi-value by
default since 1.82, which produces modules the node rejects
(`reference-types not enabled: zero byte expected`). `wasm32v1-none` is the
MVP-only, `no_std` target made for this; the SDK enables bulk memory and
sign extension on top (about 30% less gas, 10% smaller modules).
`cargo massa build` sets everything up; a plain cargo setup is
[in the docs](docs/building.md).

## Size and gas

A counter contract is about 10 KiB; a complete MRC20 (mintable, burnable,
ownable) is 27 KiB and a `transfer` costs about 370k gas, in the same range
as the AssemblyScript implementation. The SDK is `no_std`, uses a tiny
size-class allocator (a few hundred bytes), hand-rolled UTF-16/UTF-8 codecs
on the host boundary, and only pulls in what a contract uses (host imports
included). Details and knobs: [docs/size-and-gas.md](docs/size-and-gas.md).

## Repository layout

```
massa-sc-sdk/          the SDK crate (no_std on wasm, mock VM natively)
massa-sc-sdk-macros/   #[export] and #[derive(Serializable)]
massa-sc-standards/    MRC20 / MRC721 / MRC1155 / ownership / access control
cargo-massa/           build, check and scaffold tool
tools/massa-sc-runner/ local runner on top of massa-sc-runtime (separate workspace)
docs/                  reference documentation
```

## Development

```sh
cargo test --workspace                       # SDK + standards + tool, natively
cargo build -p massa-sc-sdk --target wasm32v1-none
cargo run -p cargo-massa -- massa check build/my_contract.wasm
(cd tools/massa-sc-runner && cargo build --release)   # pulls wasmer
interop/build.sh && (cd tools/massa-sc-runner && cargo test --release --test interop)   # AS interop suite
```

Minimum supported Rust version: 1.85. License: MIT.
