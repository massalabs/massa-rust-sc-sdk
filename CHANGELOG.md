# Changelog

## 0.2.0

Complete rewrite. The 0.1 prototype targeted the protobuf "wasmv1" ABI,
which Massa disabled (MIP-0002) without ever deploying a contract with it.

* Targets the live Massa ABI (the one AssemblyScript contracts use): memory
  protocol (`__new`/`__pin`, length-prefixed buffers, UTF-16 strings), every
  host import (`massa_sc_sdk::env`), C-Layer `hash_mimc`, MIP-0002 paginated
  datastore keys.
* `Args` byte-compatible with `@massalabs/as-types` / `massa-web3`,
  `#[derive(Serializable)]`, `U256`.
* High-level modules mirroring the AssemblyScript SDK: `storage`, `context`,
  `coins`, `contract`, `deferred_calls`, `op_datastore`, `crypto`, `events`,
  `collections::PersistentMap`, `helpers` (key managers), `security::AccessControl`.
* `#[export]` attribute generating the host-facing wrappers; functions keep
  their Rust signature for tests.
* Native mock VM (`massa_sc_sdk::mock`) so contracts are unit-tested with
  `cargo test`.
* Size-class allocator, abort-based panic handler, hand-optimized host codecs.
* `massa-sc-standards`: MRC20 (+ mintable, burnable, client), MRC721
  (+ metadata, enumerable), MRC1155 (+ metadata, mintable, burnable),
  ownership, role-based access control; export macros.
* `cargo-massa` (`new`, `build`, `check`) and `massa-sc-runner`.
* Builds target `wasm32v1-none`: modern `wasm32-unknown-unknown` output is
  rejected by the Massa runtime.
