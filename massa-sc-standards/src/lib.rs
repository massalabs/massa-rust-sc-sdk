//! # Massa smart-contract standards for Rust
//!
//! Reusable, audited-layout implementations of the Massa token and access
//! standards, ports of [`@massalabs/sc-standards`](https://github.com/massalabs/massa-standards)
//! with byte-identical storage layouts and entry-point names, so Rust
//! contracts interoperate with the AssemblyScript ecosystem and `massa-web3`.
//!
//! | module | standard | export macro(s) |
//! |--------|----------|-----------------|
//! | [`ownership`] | Ownable | `export_ownable!` |
//! | [`access_control`] | role-based access control | `export_access_control!` |
//! | [`mrc20`] | fungible token (ERC20) | `export_mrc20!`, `export_mrc20_mintable!`, `export_mrc20_burnable!` |
//! | [`mrc721`] | NFT (ERC721) | `export_mrc721!`, `export_mrc721_metadata!`, `export_mrc721_enumerable!` |
//! | [`mrc1155`] | multi-token (ERC1155) | `export_mrc1155!`, `export_mrc1155_metadata!`, `export_mrc1155_mintable!`, `export_mrc1155_burnable!` |
//!
//! Each module offers three layers: typed `internals` to build custom logic
//! on, `entrypoints` taking serialized [`massa_sc_sdk::Args`], and an export
//! macro that generates the `#[export]` wrappers in *your* contract crate
//! (so you pick exactly which functions are exposed). A complete token:
//!
//! ```ignore
//! #![cfg_attr(target_arch = "wasm32", no_std)]
//! use massa_sc_sdk::prelude::*;
//! use massa_sc_standards::{export_mrc20, export_mrc20_burnable, export_mrc20_mintable, export_ownable, mrc20};
//!
//! #[export]
//! pub fn constructor(mut args: Args) {
//!     let name: String = args.must_next("name");
//!     let symbol: String = args.must_next("symbol");
//!     let decimals: u8 = args.must_next("decimals");
//!     let supply: U256 = args.must_next("totalSupply");
//!     mrc20::constructor(&name, &symbol, decimals, supply);
//! }
//!
//! export_mrc20!();
//! export_mrc20_mintable!();
//! export_mrc20_burnable!();
//! export_ownable!();
//! ```
//!
//! Differences with the AssemblyScript implementation are deliberate
//! hardenings: `burn` on MRC721 requires the caller to be authorized on the
//! token, and `increaseAllowance` saturates instead of wrapping.
#![cfg_attr(target_arch = "wasm32", no_std)]

extern crate alloc;

pub mod access_control;
pub mod mrc1155;
pub mod mrc20;
pub mod mrc721;
pub mod ownership;

pub use massa_sc_sdk;
