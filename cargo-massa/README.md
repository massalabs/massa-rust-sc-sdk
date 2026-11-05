# cargo-massa

`cargo massa`: build, check and scaffold Massa smart contracts written in Rust.

```sh
cargo install --git https://github.com/massalabs/massa-rust-sc-sdk cargo-massa
rustup target add wasm32v1-none
cargo install wasm-opt            # optional, recommended

cargo massa new my-contract [--template counter|token|empty]
cargo massa build [-p pkg] [--out-dir build] [--no-wasm-opt] [--debug] [-v]
cargo massa check build/my_contract.wasm
```

`build` compiles for `wasm32v1-none` with the right target features and path
remapping, strips custom sections, runs `wasm-opt -Os` when available and
validates the module against the Massa node's feature set, host imports and
limits. `check` runs the same validation on any `.wasm` (AssemblyScript ones
too). See [docs/building.md](../docs/building.md).
