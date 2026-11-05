//! # Massa smart-contract SDK for Rust
//!
//! Write [Massa](https://massa.net) (and [C-Layer](https://github.com/massalabs/c-layer))
//! smart contracts in Rust, compiled to small, gas-efficient WebAssembly
//! modules that the Massa runtime executes exactly like AssemblyScript ones.
//!
//! ```
//! #![cfg_attr(target_arch = "wasm32", no_std)]
//! use massa_sc_sdk::prelude::*;
//!
//! const COUNTER: &[u8] = b"counter";
//!
//! /// Called once by the deployer, with write access to the new contract.
//! #[export]
//! pub fn constructor(mut args: Args) {
//!     assert!(context::is_deploying_contract(), "not deploying");
//!     let initial: u64 = args.next().unwrap_or(0);
//!     storage::set_value(COUNTER, &initial);
//! }
//!
//! /// `increment(by: u64 = 1) -> u64`
//! #[export]
//! pub fn increment(mut args: Args) -> u64 {
//!     let by: u64 = args.next().unwrap_or(1);
//!     let value = storage::get_value::<u64>(COUNTER) + by;
//!     storage::set_value(COUNTER, &value);
//!     generate_event(&format!("counter is now {value}"));
//!     value
//! }
//!
//! // Natively, the same functions run against the in-memory mock VM:
//! # use massa_sc_sdk::mock;
//! mock::reset();
//! mock::set_deploy_context(None);
//! constructor(Args::new().add(10u64));
//! assert_eq!(increment(Args::new().add(5u64)), 15);
//! assert_eq!(mock::events(), vec!["counter is now 15"]);
//! ```
//!
//! * [`storage`], [`context`], [`coins`], [`contract`], [`deferred_calls`],
//!   [`op_datastore`], [`crypto`], [`events`]: typed wrappers over the host ABI,
//!   organised like the AssemblyScript SDK namespaces.
//! * [`Args`]: argument (de)serialization compatible with `massa-web3` and `as-types`.
//! * [`Address`], [`context::Slot`], [`U256`].
//! * [`collections`], [`helpers`], [`security`]: persistent map, key managers,
//!   bitmask access control.
//! * [`clayer`]: C-Layer specific ABIs (MiMC).
//! * [`env`]: thin typed layer directly over every host import.
//! * [`mock`] (native targets only): in-memory VM to unit test contracts with
//!   `cargo test`.
//!
//! The crate is `no_std` on wasm. The [`prelude`] re-exports the usual `alloc`
//! items (`Vec`, `String`, `format!`, `vec!`...) so contracts don't need to.
#![cfg_attr(target_arch = "wasm32", no_std)]
#![warn(missing_docs)]
#![allow(clippy::new_without_default)]

extern crate alloc;

#[cfg(target_arch = "wasm32")]
mod abi;
pub mod address;
pub mod allocator;
pub mod args;
pub mod clayer;
#[doc(hidden)]
pub mod codec;
pub mod coins;
pub mod collections;
pub mod constants;
pub mod context;
pub mod contract;
pub mod crypto;
pub mod deferred_calls;
pub mod env;
pub mod events;
pub mod helpers;
#[cfg(target_arch = "wasm32")]
mod memory;
#[cfg(not(target_arch = "wasm32"))]
pub mod mock;
pub mod op_datastore;
mod panic;
pub mod security;
pub mod storage;

#[doc(hidden)]
pub mod entrypoint;
pub mod prelude;

pub use address::Address;
pub use args::{Args, ArgsDecode, ArgsEncode, ArgsError, Serializable, I256, U256};
pub use context::Slot;
pub use events::{create_event, generate_event, generate_raw_event};
pub use panic::abort;

#[cfg(feature = "macros")]
pub use massa_sc_sdk_macros::{export, Serializable};

/// Concatenate string pieces without going through `core::fmt`
/// (keeps formatting code out of contracts that never format).
#[doc(hidden)]
pub fn concat_str(parts: &[&str]) -> alloc::string::String {
    let mut s = alloc::string::String::with_capacity(parts.iter().map(|p| p.len()).sum());
    for p in parts {
        s.push_str(p);
    }
    s
}

/// Alias of [`abort`] matching the AssemblyScript vocabulary.
#[inline(always)]
pub fn throw(message: &str) -> ! {
    abort(message)
}
