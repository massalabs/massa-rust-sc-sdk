# massa-sc-runner

Execute compiled Massa smart contracts locally, inside the real Massa engine
(`massa-sc-runtime`: wasmer 4.3.6, gas metering, node limits), against an
in-memory ledger. Several contracts can be deployed and call each other, so
it doubles as a test bench for cross-contract and cross-SDK scenarios
(AssemblyScript modules run the same way). No node needed.

```sh
cargo install --path .                                           # Massa ABI
cargo install --path . --no-default-features --features clayer   # C-Layer ABI (adds hash_mimc)

massa-sc-runner build/counter.wasm constructor --args u64:10 --deploy \; increment --args u64:5 \; get
massa-sc-runner build/token.wasm constructor --args "s:My Token,s:MTK,u8:9,u256:1000000" --deploy \
    \; transfer --args "addr:AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq,u256:10" --coins 10000000
massa-sc-runner build/deployer.wasm --opds "hex:00=args:u64:1" --opds "args:u64:1=@build/child.wasm" main

# several contracts: a Rust token driven by an AssemblyScript caller, then swap the token implementation
massa-sc-runner build/rs_token.wasm --contract caller=build/as_caller.wasm \
    constructor --args "s:T,s:T,u8:9,u256:1000" --deploy \
    \; transfer --args "addr:@caller,u256:100" \
    \; tokenBalance --at @caller --args "addr:@main,addr:@caller" \
    \; --swap @main=build/as_token.wasm \
    \; balanceOf --args "addr:@caller"
```

Argument spec types: `s:`/`addr:` (string, `@name` resolves to a `--contract`
address), `bool:`, `u8:`..`u64:`, `i8:`..`i64:`, `u128:`, `u256:`, `f32:`,
`f64:`, `bytes:<hex>` (length-prefixed), `hex:<raw>`.

## As a library

```rust
use massa_sc_runner::{Runner, DEFAULT_CALLER};

let mut r = Runner::new();
let token = r.deploy_new(&std::fs::read("build/token.wasm")?, &constructor_args, coins)?;
let out = r.call(&token, "balanceOf", &args)?;          // Outcome { ret, gas, events, async_messages }
r.call_from(other_address, &token, "transfer", &args, coins)?;
r.set_bytecode(&token, &other_implementation)?;         // upgrade on top of the datastore
let state = r.datastore(&token);
```

The interoperability suite (`tests/interop.rs`, see
[docs/interop.md](../docs/interop.md)) is built on it.

## Notes

* The ledger model is deliberately simple (no storage costs charged, no real
  signature checks); failed calls are not rolled back. It is a development
  tool, not a consensus reference.
* This crate is a standalone workspace: it pulls wasmer and
  `massa-sc-runtime`. The runtime's `Interface` trait still carries the
  disabled `wasmv1` methods, whose types come from `massa-proto-rs`; that is
  why protobuf appears here, and only here. The SDK, the standards and the
  contracts have no protobuf dependency.
