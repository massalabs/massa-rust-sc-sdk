# Contributing

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p massa-sc-sdk -p massa-sc-standards --target wasm32v1-none
```

Interoperability with AssemblyScript (fixtures need npm):

```sh
SC_STANDARDS=path/to/massa-standards/smart-contracts interop/build.sh
(cd tools/massa-sc-runner && cargo test --release --test interop)
```

End-to-end: build an example from
[massa-rust-sc-examples](https://github.com/massalabs/massa-rust-sc-examples)
with `cargo massa build` and run it with `tools/massa-sc-runner`, which
executes modules in the real `massa-sc-runtime`.

Host ABI changes land in [massa-sc-runtime](https://github.com/massalabs/massa-sc-runtime)
(`src/as_execution/context.rs`); mirror them in `massa-sc-sdk/src/abi.rs`,
`env.rs`, the mock, `cargo-massa/src/abi.rs` and `docs/abi.md`.
