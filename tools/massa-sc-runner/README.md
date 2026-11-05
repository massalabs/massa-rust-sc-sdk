# massa-sc-runner

Execute a compiled Massa smart contract locally, inside the real Massa engine
(`massa-sc-runtime`: wasmer 4.3.6, gas metering, node limits), against an
in-memory ledger. No node needed; prints return values, events, gas and the
final datastore.

```sh
cargo install --path .                        # Massa ABI
cargo install --path . --no-default-features --features clayer   # C-Layer ABI (adds hash_mimc)

massa-sc-runner build/counter.wasm constructor --args u64:10 --deploy \; increment --args u64:5 \; get
massa-sc-runner build/token.wasm constructor --args "s:My Token,s:MTK,u8:9,u256:1000000" --deploy \
    \; transfer --args "addr:AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq,u256:10" --coins 10000000
massa-sc-runner build/deployer.wasm --opds "hex:00=args:u64:1" --opds "args:u64:1=@build/child.wasm" main
```

Argument spec types: `s:`/`addr:` (string), `bool:`, `u8:`..`u64:`, `i8:`..`i64:`,
`u128:`, `u256:`, `f32:`, `f64:`, `bytes:<hex>` (length-prefixed), `hex:<raw>`.

The ledger model is deliberately simple (no storage costs, no real
signatures); it is a development tool, not a consensus reference. This crate
is a standalone workspace because it pulls wasmer.
