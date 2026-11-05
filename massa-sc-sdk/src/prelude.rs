//! Everything a contract usually needs: `use massa_sc_sdk::prelude::*;`
//!
//! Re-exports the SDK modules (`storage`, `context`, ...), the main types
//! (`Args`, `Address`, `U256`, `Slot`), the `#[export]` attribute, and the
//! `alloc` items (`Vec`, `String`, `format!`, `vec!`...) that a `no_std`
//! contract would otherwise have to import by hand.

pub use alloc::borrow::ToOwned;
pub use alloc::boxed::Box;
pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
pub use alloc::string::{String, ToString};
pub use alloc::vec::Vec;
pub use alloc::{format, vec};

pub use crate::address::Address;
pub use crate::args::{Args, ArgsDecode, ArgsEncode, ArgsError, Serializable, I256, U256};
pub use crate::context::Slot;
pub use crate::events::{create_event, generate_event, generate_raw_event};
pub use crate::storage::StorageValue;
pub use crate::{abort, throw};
pub use crate::{
    clayer, coins, constants, context, contract, crypto, deferred_calls, events, op_datastore, storage,
};

#[cfg(feature = "macros")]
pub use massa_sc_sdk_macros::{export, Serializable};
