# Memory protocol and runtime model

## How the host talks to the module

Massa's runtime executes "AssemblyScript modules": any wasm module that
follows the AssemblyScript object layout for the values crossing the host
boundary. A Rust module built with this SDK follows it:

* A **buffer** (`StaticArray<u8>` for the host) is a pointer to its payload;
  the payload byte length is a little-endian `u32` stored in the 4 bytes
  *before* the pointer.
* A **string** is the same with a UTF-16LE payload (byte length = 2 × units).
* The host allocates module memory by calling the exported
  `__new(size: i32, class_id: i32) -> i32`, writes the payload at the
  returned pointer and expects the length header at `ptr - 4`. For strings it
  also calls `__pin(ptr)`. `__unpin` and `__collect` are exported as no-ops.
* An entry point receives its serialized parameter as such a buffer pointer
  and returns a buffer pointer (or nothing). Everything is `i32`/`i64` at the
  wasm level.

`massa_sc_sdk::memory` implements this: `HostBuf` builds a temporary
`[len][payload]` block for every buffer/string passed to a host function and
frees it right after the call; `take_bytes`/`take_string` copy a host-provided
object out and release the block `__new` had allocated; return values are
written in a block that is never freed (the instance is dropped when the
call ends).

The `#[export]` attribute generates, for `wasm32` only:

```rust
#[export_name = "increment"]
pub extern "C" fn __massa_export_increment(arg: i32) -> i32 {
    let param = take_param(arg);                          // Vec<u8>
    give_return(increment(FromParam::from_param(param)))  // -> buffer pointer
}
```

The Rust function keeps its signature, so tests call it directly.

## Allocator

Executions are single-threaded, short-lived and capped at 4 MiB, so the SDK
ships a size-class bump allocator (`allocator` feature, on by default):

* sizes are rounded to power-of-two classes from 16 B to 512 KiB with
  per-class free lists (O(1) allocate/free, memory is recycled);
* freeing or growing the most recent allocation moves the bump pointer, so
  the common LIFO pattern and `Vec`/`String` growth cost nothing;
* larger blocks are bump-allocated with an exact size;
* memory is grown with `memory.grow` on demand and never returned.

It costs a few hundred bytes of code and a handful of instructions per
allocation. Disable the feature to plug another `#[global_allocator]`.

## Stack and memory budget

Rust's default shadow stack is 1 MiB; `cargo massa build` sets it to 256 KiB
(`--stack-size`), leaving about 3.7 MiB of heap. The data segment and the
stack live below `__heap_base`; the heap starts there.

## Panics

A `panic!` (so any `assert!`, `unwrap()`, overflow with `overflow-checks`,
out-of-bounds index...) calls the host `abort` with the message and the
location: the execution stops, every state change is reverted, and the
operation event reads `error: <message> at <file>:<line> col: <col>`. The
`panic-message` feature (default) renders formatted messages; without it only
literal messages are reported, which saves ~1 KiB of `core::fmt` when the
contract does not format text otherwise. `massa_sc_sdk::abort(msg)` aborts
explicitly.

## Determinism

The runtime disables threads, SIMD, non-deterministic float behaviour (NaN
canonicalization) and anything time/randomness related outside the ABI.
Rust code is deterministic as long as it does not read uninitialized memory,
which safe Rust guarantees. Floating point is allowed (`f32`/`f64` in
`Args`) but avoid it for anything of value.

## Native builds

On non-wasm targets the same `env` functions are served by
`massa_sc_sdk::mock`, a thread-local in-memory VM: ledger (balances,
bytecode, datastores), call stack, coins, slot/time/chain id, events,
operation datastore, mocked calls to other contracts, deferred calls and
asynchronous messages. The memory protocol and allocator are not involved.
