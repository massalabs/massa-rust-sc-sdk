# Building contracts

## With `cargo massa` (recommended)

```sh
rustup target add wasm32v1-none
cargo install --git https://github.com/massalabs/massa-rust-sc-sdk cargo-massa
cargo install wasm-opt        # optional but worth it

cargo massa build             # every cdylib of the workspace -> build/<crate>.wasm
cargo massa build -p my-token --out-dir dist
cargo massa check build/my_token.wasm
```

`cargo massa build`:

1. runs `cargo build --release --target wasm32v1-none` with
   `-C target-feature=+bulk-memory,+sign-ext`, a 256 KiB shadow stack
   (`--stack-size`), and `--remap-path-prefix` flags so no local path ends up
   in the binary (panic locations become `/src/...`, `/cargo/...`);
2. strips every custom section (`name`, `producers`, `target_features`,
   DWARF): the node allows at most one;
3. runs `wasm-opt -Os` when it is installed, restricted to the features the
   Massa runtime enables (`--no-wasm-opt`, `--wasm-opt-level -Oz`);
4. validates the module exactly like the node would (feature set, host
   imports and their signatures, function/import/export/section limits,
   memory size, entry-point signatures) and prints a report.

`--network massa|clayer|any` turns the warnings about network-specific
imports (`mimc` is C-Layer only, paginated datastore keys need MIP-0002 which
C-Layer does not have yet) into errors for the network you target.

Extra rustc flags: `--rustflag` or the `MASSA_RUSTFLAGS` environment
variable. Extra cargo flags after `--`.

`cargo massa build` sets `CARGO_ENCODED_RUSTFLAGS`, which replaces any
`rustflags` from `.cargo/config.toml` for that build.

## With plain cargo

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]      # rlib: lets `cargo test` link the crate natively

[profile.release]
opt-level = "s"         # "z" is ~7% smaller but ~20% more gas; "3" barely beats "s" on gas
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true            # removes the custom sections the node would reject
overflow-checks = true  # keep arithmetic safe in release
```

```toml
# .cargo/config.toml
[alias]
wasm = "build --release --target wasm32v1-none"

[target.wasm32v1-none]
rustflags = ["-C", "target-feature=+bulk-memory,+sign-ext", "-C", "link-arg=-zstack-size=262144"]
```

Then `cargo wasm` produces `target/wasm32v1-none/release/<crate>.wasm`.
Do not set `[build] target`: it would make `cargo test` target wasm.

## Why not `wasm32-unknown-unknown`

Since Rust 1.82 the `wasm32-unknown-unknown` target enables the
reference-types and multi-value proposals by default, and the precompiled
`core`/`alloc` use them (`call_indirect` gets an over-long table index
encoding). Massa's runtime (wasmer 4.3.6 with a fixed, deterministic feature
set: bulk memory only) rejects such modules at compilation:
`reference-types not enabled: zero byte expected`. `wasm32v1-none` is the
MVP-only target; it is `no_std`, which the SDK is anyway.

If you must use `wasm32-unknown-unknown` (older toolchains), run the output
through `wasm-opt --mvp-features --enable-bulk-memory --enable-sign-ext
--enable-mutable-globals` and check it with `cargo massa check`.

## Targets, toolchain

* MSRV 1.85 (`wasm32v1-none` is tier 2 since 1.84).
* The SDK and standards are `no_std` on wasm and `std` on native targets,
  where the host ABI is served by the in-memory mock VM: `cargo test` just
  works, with `#[should_panic]` and all.
* Contracts should carry `#![cfg_attr(target_arch = "wasm32", no_std)]` and
  import `massa_sc_sdk::prelude::*`, which re-exports `Vec`, `String`,
  `format!`, `vec!`... so the same code compiles on both.

## Running locally without a node

`tools/massa-sc-runner` executes a module in the real runtime
(`massa-sc-runtime`) against an in-memory ledger, printing return values,
events, gas and the final datastore:

```sh
cargo install --path tools/massa-sc-runner          # or tools/massa-sc-runner/clayer for the C-Layer ABI
massa-sc-runner build/counter.wasm constructor --args u64:10 --deploy \; increment --args u64:5 \; get
```
