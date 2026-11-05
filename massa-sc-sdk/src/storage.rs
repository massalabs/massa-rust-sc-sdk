//! Persistent key/value datastore of the current contract (and, when the
//! caller has write access, of other addresses).
//!
//! Keys and values are plain bytes: anything `AsRef<[u8]>` works as a key or
//! value (`&str`, `String`, `&[u8]`, `Vec<u8>`, `[u8; N]`, [`Args`], ...).
//! Typed values can be stored through [`Args`] (`set(KEY, Args::new().add(v))`)
//! or with [`set_value`] / [`get_value`], which use the [`StorageValue`]
//! encoding (integers little-endian, strings raw UTF-8, structs serialized).
//!
//! Every datastore write is paid in coins by the contract (storage cost), and
//! every failed read (`get` of a missing key) aborts the execution: use
//! [`try_get`] / [`has`] when a key may be absent.
//!
//! Writing to another address (`*_of` functions) is only possible while the
//! current execution has write access to it, i.e. in the same execution that
//! created it with [`crate::contract::create_sc`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::args::{Args, ArgsDecode, ArgsEncode, Serializable, U256};
use crate::constants::MAX_DATASTORE_KEYS_PAGE;
use crate::{env, Address};

/// Set `key` to `value` in the current contract datastore (created or overwritten).
#[inline]
pub fn set(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
    env::set(key.as_ref(), value.as_ref())
}

/// Set `key` to `value` in the datastore of `address` (requires write access).
#[inline]
pub fn set_of(address: &Address, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
    env::set_of(address.as_str(), key.as_ref(), value.as_ref())
}

/// Value stored at `key`; aborts when the key does not exist.
#[inline]
pub fn get(key: impl AsRef<[u8]>) -> Vec<u8> {
    env::get(key.as_ref())
}

/// Value stored at `key` in the datastore of `address`; aborts when missing.
#[inline]
pub fn get_of(address: &Address, key: impl AsRef<[u8]>) -> Vec<u8> {
    env::get_of(address.as_str(), key.as_ref())
}

/// Value stored at `key`, or `None` when the key does not exist.
#[inline]
pub fn try_get(key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
    let key = key.as_ref();
    if env::has(key) {
        Some(env::get(key))
    } else {
        None
    }
}

/// Value stored at `key` in the datastore of `address`, or `None`.
#[inline]
pub fn try_get_of(address: &Address, key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
    let key = key.as_ref();
    if env::has_of(address.as_str(), key) {
        Some(env::get_of(address.as_str(), key))
    } else {
        None
    }
}

/// Value stored at `key`, or `default` when the key does not exist.
#[inline]
pub fn get_or(key: impl AsRef<[u8]>, default: impl Into<Vec<u8>>) -> Vec<u8> {
    try_get(key).unwrap_or_else(|| default.into())
}

/// `true` when `key` exists in the current contract datastore.
#[inline]
pub fn has(key: impl AsRef<[u8]>) -> bool {
    env::has(key.as_ref())
}

/// `true` when `key` exists in the datastore of `address`.
#[inline]
pub fn has_of(address: &Address, key: impl AsRef<[u8]>) -> bool {
    env::has_of(address.as_str(), key.as_ref())
}

/// Delete `key`; aborts when it does not exist (see [`del_if_exists`]).
#[inline]
pub fn del(key: impl AsRef<[u8]>) {
    env::del(key.as_ref())
}

/// Delete `key` when present; returns whether it was.
#[inline]
pub fn del_if_exists(key: impl AsRef<[u8]>) -> bool {
    let key = key.as_ref();
    if env::has(key) {
        env::del(key);
        true
    } else {
        false
    }
}

/// Delete `key` from the datastore of `address` (requires write access).
#[inline]
pub fn del_of(address: &Address, key: impl AsRef<[u8]>) {
    env::delete_of(address.as_str(), key.as_ref())
}

/// Append `value` to the existing value at `key`; aborts when the key is missing.
#[inline]
pub fn append(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
    env::append(key.as_ref(), value.as_ref())
}

/// Append `value` to the existing value at `key` of `address`.
#[inline]
pub fn append_of(address: &Address, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
    env::append_of(address.as_str(), key.as_ref(), value.as_ref())
}

// ---------------------------------------------------------------------------
// typed helpers
// ---------------------------------------------------------------------------

/// Value encoding used by [`set_value`], [`get_value`] and
/// [`crate::collections::PersistentMap`]: compact, with no length prefix for
/// strings and byte strings (same layout as the AssemblyScript helpers
/// `Storage.set<string>`, `PersistentMap`, `u64ToBytes`, `boolToByte`...).
pub trait StorageValue: Sized {
    fn to_storage(&self) -> Vec<u8>;
    fn from_storage(bytes: &[u8]) -> Option<Self>;
}

macro_rules! storage_value_via_args {
    ($($t:ty),* $(,)?) => {$(
        impl StorageValue for $t {
            #[inline]
            fn to_storage(&self) -> Vec<u8> { self.encode() }
            #[inline]
            fn from_storage(bytes: &[u8]) -> Option<Self> { Self::decode_bytes(bytes).ok() }
        }
    )*};
}
storage_value_via_args!(bool, u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, f32, f64, U256);

impl StorageValue for String {
    fn to_storage(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn from_storage(bytes: &[u8]) -> Option<Self> {
        core::str::from_utf8(bytes).ok().map(String::from)
    }
}

impl StorageValue for Vec<u8> {
    fn to_storage(&self) -> Vec<u8> {
        self.clone()
    }
    fn from_storage(bytes: &[u8]) -> Option<Self> {
        Some(bytes.to_vec())
    }
}

impl StorageValue for Address {
    fn to_storage(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn from_storage(bytes: &[u8]) -> Option<Self> {
        String::from_storage(bytes).map(Address::from)
    }
}

impl StorageValue for Args {
    fn to_storage(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn from_storage(bytes: &[u8]) -> Option<Self> {
        Some(Args::from(bytes))
    }
}

/// Store `value` at `key` with the [`StorageValue`] encoding.
#[inline]
pub fn set_value<V: StorageValue>(key: impl AsRef<[u8]>, value: &V) {
    env::set(key.as_ref(), &value.to_storage())
}

/// Read a [`StorageValue`] at `key`; aborts when the key is missing or the
/// stored bytes do not decode as `V`.
#[inline]
pub fn get_value<V: StorageValue>(key: impl AsRef<[u8]>) -> V {
    match V::from_storage(&env::get(key.as_ref())) {
        Some(v) => v,
        None => crate::abort("invalid stored value"),
    }
}

/// Read a [`StorageValue`] at `key`, `None` when missing or undecodable.
#[inline]
pub fn try_get_value<V: StorageValue>(key: impl AsRef<[u8]>) -> Option<V> {
    try_get(key).and_then(|b| V::from_storage(&b))
}

/// Store a [`Serializable`] structure at `key` (its [`Args`] encoding).
#[inline]
pub fn set_serializable<T: Serializable>(key: impl AsRef<[u8]>, value: &T) {
    env::set(key.as_ref(), &value.serialize())
}

/// Read a [`Serializable`] structure at `key`; aborts when missing or invalid.
#[inline]
pub fn get_serializable<T: Serializable>(key: impl AsRef<[u8]>) -> T {
    match T::deserialize(&env::get(key.as_ref())) {
        Ok(v) => v,
        Err(e) => crate::abort(&alloc::format!("invalid stored value: {e}")),
    }
}

/// Read a [`Serializable`] structure at `key`, `None` when missing or invalid.
#[inline]
pub fn try_get_serializable<T: Serializable>(key: impl AsRef<[u8]>) -> Option<T> {
    try_get(key).and_then(|b| T::deserialize(&b).ok())
}

// ---------------------------------------------------------------------------
// keys
// ---------------------------------------------------------------------------

/// All keys of the current datastore starting with `prefix` (empty = all).
///
/// Deprecated usage from execution version 2 (MIP-0002): the call fails when
/// more than [`MAX_DATASTORE_KEYS_PAGE`] keys match. Prefer [`get_keys_page`]
/// for any datastore that can grow.
#[inline]
pub fn get_keys(prefix: impl AsRef<[u8]>) -> Vec<Vec<u8>> {
    env::get_keys(prefix.as_ref())
}

/// All keys of the datastore of `address` starting with `prefix`.
/// Same limitation as [`get_keys`].
#[inline]
pub fn get_keys_of(address: &Address, prefix: impl AsRef<[u8]>) -> Vec<Vec<u8>> {
    env::get_keys_of(address.as_str(), prefix.as_ref())
}

/// One page of keys of the current datastore, in ascending byte order.
///
/// * `prefix`: only keys starting with it (empty matches every key).
/// * `start_key`: exclusive cursor, only keys strictly greater are returned
///   (empty starts at the beginning). Pass the last key of the previous page.
/// * `count`: page size in `1..=MAX_DATASTORE_KEYS_PAGE`.
///
/// Each call costs a flat amount of gas whatever `count`. Process a page then
/// drop it; when the work does not fit in one execution, store the cursor and
/// resume later. Available from execution version 2 (MIP-0002): a contract
/// importing it cannot be instantiated on a node where MIP-0002 is not active.
///
/// ```ignore
/// let mut cursor = Vec::new();
/// loop {
///     let page = storage::get_keys_page(b"user:", &cursor, MAX_DATASTORE_KEYS_PAGE);
///     for key in &page { /* ... */ }
///     if page.len() < MAX_DATASTORE_KEYS_PAGE as usize { break; }
///     cursor = page.last().unwrap().clone();
/// }
/// ```
#[inline]
pub fn get_keys_page(prefix: impl AsRef<[u8]>, start_key: impl AsRef<[u8]>, count: u32) -> Vec<Vec<u8>> {
    env::get_keys_page(prefix.as_ref(), start_key.as_ref(), count)
}

/// One page of keys of the datastore of `address`; see [`get_keys_page`].
#[inline]
pub fn get_keys_of_page(
    address: &Address,
    prefix: impl AsRef<[u8]>,
    start_key: impl AsRef<[u8]>,
    count: u32,
) -> Vec<Vec<u8>> {
    env::get_keys_of_page(address.as_str(), prefix.as_ref(), start_key.as_ref(), count)
}

/// Iterate over every key starting with `prefix`, page by page, calling `f`
/// for each key. Returns the number of keys visited. Prefer resuming from a
/// stored cursor over visiting a large datastore in one execution.
pub fn for_each_key(prefix: impl AsRef<[u8]>, mut f: impl FnMut(&[u8])) -> usize {
    let prefix = prefix.as_ref();
    let mut cursor: Vec<u8> = Vec::new();
    let mut visited = 0;
    loop {
        let page = env::get_keys_page(prefix, &cursor, MAX_DATASTORE_KEYS_PAGE);
        for key in &page {
            f(key);
        }
        visited += page.len();
        if page.len() < MAX_DATASTORE_KEYS_PAGE as usize {
            return visited;
        }
        cursor = page.into_iter().next_back().unwrap_or_default();
    }
}

/// Storage cost (nanoMAS) of a new datastore entry with a key of `key_len`
/// bytes and a value of `value_len` bytes.
pub const fn entry_cost(key_len: usize, value_len: usize) -> u64 {
    (crate::constants::DATASTORE_ENTRY_BASE_SIZE + key_len + value_len) as u64
        * crate::constants::STORAGE_BYTE_COST
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn raw_and_typed_access() {
        mock::reset();
        assert!(!has("k"));
        assert_eq!(try_get("k"), None);
        set("k", "v");
        assert_eq!(get("k"), b"v");
        append("k", "!");
        assert_eq!(get_or("k", b"x".to_vec()), b"v!");
        assert!(del_if_exists("k"));
        assert!(!del_if_exists("k"));

        set_value("n", &42u64);
        assert_eq!(get("n"), 42u64.to_le_bytes());
        assert_eq!(get_value::<u64>("n"), 42);
        set_value("s", &String::from("hey"));
        assert_eq!(get("s"), b"hey");
        assert_eq!(try_get_value::<String>("s").as_deref(), Some("hey"));
        assert_eq!(try_get_value::<u64>("missing"), None);
    }

    #[test]
    fn key_pagination() {
        mock::reset();
        for i in 0..1203u32 {
            set(alloc::format!("p{i:05}"), [1]);
        }
        set("other", [2]);
        let mut n = 0;
        let total = for_each_key("p", |k| {
            assert!(k.starts_with(b"p"));
            n += 1;
        });
        assert_eq!((n, total), (1203, 1203));
        let first = get_keys_page("p", "", 2);
        assert_eq!(first, vec![b"p00000".to_vec(), b"p00001".to_vec()]);
        let next = get_keys_page("p", &first[1], 1);
        assert_eq!(next, vec![b"p00002".to_vec()]);
        assert_eq!(get_keys("other").len(), 1);
    }
}
