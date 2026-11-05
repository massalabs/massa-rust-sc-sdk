//! Higher-level persistent collections built on [`crate::storage`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
use core::marker::PhantomData;

use crate::storage::{self, StorageValue};

/// A persistent map stored in the contract datastore under a unique prefix.
///
/// Keys are rendered with `Display` and stored as `"<prefix>::<key>"`; values
/// use the [`StorageValue`] encoding. The map does not keep track of its keys:
/// store them in the values if you need to enumerate them, or use
/// [`crate::storage::get_keys_page`] with the map prefix.
///
/// Every collection living in the same contract must use a distinct prefix.
///
/// ```ignore
/// let balances: PersistentMap<Address, u64> = PersistentMap::new("bal");
/// balances.set(&user, 10);
/// assert_eq!(balances.get(&user), Some(10));
/// ```
pub struct PersistentMap<K, V> {
    prefix: String,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

const KEY_SEPARATOR: &str = "::";

impl<K: Display, V: StorageValue> PersistentMap<K, V> {
    /// Create or reopen the map stored under `prefix`.
    pub fn new(prefix: impl Into<String>) -> Self {
        let mut prefix = prefix.into();
        prefix.push_str(KEY_SEPARATOR);
        Self { prefix, _k: PhantomData, _v: PhantomData }
    }

    /// Datastore key of `key`.
    pub fn storage_key(&self, key: &K) -> Vec<u8> {
        alloc::format!("{}{}", self.prefix, key).into_bytes()
    }

    /// Datastore prefix shared by every entry (for key enumeration).
    pub fn prefix(&self) -> &[u8] {
        self.prefix.as_bytes()
    }

    pub fn contains(&self, key: &K) -> bool {
        storage::has(self.storage_key(key))
    }

    /// Value for `key`, `None` when absent.
    pub fn get(&self, key: &K) -> Option<V> {
        storage::try_get_value(self.storage_key(key))
    }

    /// Value for `key`, or `default`.
    pub fn get_or(&self, key: &K, default: V) -> V {
        self.get(key).unwrap_or(default)
    }

    /// Value for `key`; aborts with `msg` when absent.
    pub fn get_some(&self, key: &K, msg: &str) -> V {
        match self.get(key) {
            Some(v) => v,
            None => crate::abort(msg),
        }
    }

    pub fn set(&self, key: &K, value: V) {
        storage::set_value(self.storage_key(key), &value)
    }

    /// Remove `key`; returns whether it existed.
    pub fn delete(&self, key: &K) -> bool {
        storage::del_if_exists(self.storage_key(key))
    }
}

impl<K: Display, V: StorageValue + Default> PersistentMap<K, V> {
    /// Value for `key`, or `V::default()`.
    pub fn get_or_default(&self, key: &K) -> V {
        self.get(key).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mock, Address};

    #[test]
    fn map_roundtrip() {
        mock::reset();
        let m: PersistentMap<Address, u64> = PersistentMap::new("bal");
        let a = Address::from("AU1a");
        assert!(!m.contains(&a));
        assert_eq!(m.get_or_default(&a), 0);
        m.set(&a, 5);
        assert_eq!(m.get(&a), Some(5));
        assert_eq!(storage::get("bal::AU1a"), 5u64.to_le_bytes());
        assert!(m.delete(&a));
        assert!(!m.delete(&a));
        let s: PersistentMap<u32, String> = PersistentMap::new("names");
        s.set(&1, "one".into());
        assert_eq!(s.get_some(&1, "missing"), "one");
    }
}
