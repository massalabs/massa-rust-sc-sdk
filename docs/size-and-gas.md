# Size and gas

Measured with `massa-sc-runtime` (the node's engine) and the node gas table.
Gas = 23 × executed wasm instructions + per-ABI costs. Bytecode costs
0.0001 MAS per byte at deployment, so 10 KiB ≈ 1 MAS.

## Reference points

| contract | size | gas |
|---|---|---|
| empty entry point reading a `u64` argument | 9.9 KiB (6.7 with `wasm-opt`) | 40k |
| counter: `has` + `get` + `set` + static event | 10 KiB | 88k |
| `generate_event(&format!("counter is now {value}"))` | +3.5 KiB | +30k |
| `context::caller()` + event with the address | +2 KiB | +240k |
| MRC20 token, mintable + burnable + ownable | 27 KiB (`wasm-opt -Os`) | `transfer` 370k, `balanceOf` 76k |
| same token, AssemblyScript (`massa-standards`) | ~30 KiB | `transfer` ~400-600k |

The fixed part of a Rust module (panic path, allocator, `Vec` growth code,
host codecs) is larger than the AssemblyScript runtime stub (≈ 8 KiB vs 2 KiB);
per-feature code is comparable or smaller, and gas is comparable or lower.

## What costs gas

* **Every wasm instruction**: a byte-by-byte loop costs ~25 instructions per
  byte in stack-machine form. The SDK uses `memory.copy` (one instruction)
  for buffers and hand-rolled ASCII fast paths for UTF-16 strings, but
  anything that iterates characters (`format!`, `{:?}`, JSON-ish parsing) is
  proportionally expensive. Prefer bytes and fixed-width integers to text.
* **Host calls**: see [abi.md](abi.md). `call` is 15k, `create_sc` 745M,
  `send_message` 40M, `get_keys_page` 190k flat. `context::caller()` costs the
  call-stack ABI (1.5k) plus decoding ~120 UTF-16 units of JSON (~200k):
  cache it in a local instead of calling it repeatedly.
* **Storage**: every entry costs coins (`storage::entry_cost`), not gas; but
  writes are 940 gas each, reads 1040.

## Knobs

| knob | effect |
|---|---|
| `wasm-opt -Os` (default in `cargo massa build`) | −20% size, −30% gas |
| `opt-level = "s"` (default) vs `"z"` | `z` is ~7% smaller, ~20% more gas |
| `opt-level = "3"` | ~1% less gas than `s`, ~8% bigger |
| `-C target-feature=+bulk-memory,+sign-ext` (default) | −10% size, −30% gas vs plain MVP |
| `default-features = false, features = ["macros", "allocator", "panic-handler"]` | drops formatted panic messages: −1 KiB when nothing else formats text |
| `overflow-checks = false` | a few % less gas; not recommended for contracts |
| `lto = "fat"`, `codegen-units = 1` | required for small output |

## Writing lean contracts

* Keep `#![cfg_attr(target_arch = "wasm32", no_std)]`; never pull `std`.
* `format!`/`{}`/`{:?}` bring `core::fmt` (~4-6 KiB once, then cheap). If you
  only need an event with numbers, that is fine; avoid `Debug` formatting of
  strings (`{:?}` escapes every character).
* Use `Args`/`U256` little-endian encodings for storage values
  (`storage::set_value`) rather than decimal strings.
* Store addresses as bytes (`Address::as_bytes`) in keys; keep keys short
  (storage is paid per byte).
* One `transfer_remaining` per entry point, at the end, with the balance
  captured at the start.
* `cargo massa build -v` prints the imports and sizes; `twiggy`/`wasm-tools
  print` on an unstripped build (`CARGO_PROFILE_RELEASE_STRIP=false`) shows
  which functions dominate.
