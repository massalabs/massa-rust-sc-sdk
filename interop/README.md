# Rust <-> AssemblyScript interoperability suite

Fixtures and build script of the suite described in
[docs/interop.md](../docs/interop.md). The tests live in
[`tools/massa-sc-runner/tests/interop.rs`](../tools/massa-sc-runner/tests/interop.rs)
and run both sides in the real Massa runtime.

```
as/      AssemblyScript fixtures (asc + @massalabs/massa-as-sdk + @massalabs/sc-standards)
rust/    Rust fixtures (massa-sc-sdk + massa-sc-standards)
build/   output: as_<name>.wasm and rs_<name>.wasm (ignored)
```

| fixture | checks |
|---|---|
| `echo` | every `Args` type round-trips; golden vectors; return-value idioms |
| `caller` | generic calls, `functionExists`, `localCall`, MRC20 client of each SDK against the other's token, coins, call stack |
| `storage` | `Storage`, `PersistentMap`, `ConstantManager`, `MapManager`, `AccessControl` layouts |
| `token`, `nft`, `multi` | MRC20 / MRC721 / MRC1155 (+ ownership, roles): identical datastores, events, returns; implementations interchangeable |

```sh
# reference: massa-standards main
git clone --depth 1 https://github.com/massalabs/massa-standards /tmp/massa-standards
SC_STANDARDS=/tmp/massa-standards/smart-contracts ./build.sh
# or the published @massalabs/sc-standards (see docs/interop.md for the one known difference)
./build.sh

cd ../tools/massa-sc-runner && cargo test --release --test interop
```

Requirements: npm, the `wasm32v1-none` target, `cargo-massa` is run from the
workspace (`cargo run -p cargo-massa`).
