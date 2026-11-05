//! Storage key management helpers (ports of the AssemblyScript
//! `KeyIncrementer`, `ConstantManager` and `MapManager`).
//!
//! They give each piece of state a short, unique binary key prefix so that
//! several components (e.g. an ownership module and a token module) can share
//! one datastore without collisions, while keeping keys small (storage is paid
//! per byte).

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::args::{Args, ArgsDecode, ArgsEncode};
use crate::storage;

/// Produces a sequence of unique storage keys.
pub trait KeySequence {
    /// The next unused key.
    fn next_key(&mut self) -> Vec<u8>;
}

/// A [`KeySequence`] that simply counts: keys are the [`Args`] encoding of a
/// counter (`u8` by default, so 1-byte keys).
pub struct KeyIncrementer<T = u8> {
    /// Value of the next key.
    pub counter: T,
}

impl<T> KeyIncrementer<T> {
    /// Start counting at `start`.
    pub const fn new(start: T) -> Self {
        Self { counter: start }
    }
}

impl<T: Default> Default for KeyIncrementer<T> {
    fn default() -> Self {
        Self { counter: T::default() }
    }
}

macro_rules! impl_incrementer {
    ($($t:ty),*) => {$(
        impl KeySequence for KeyIncrementer<$t> {
            /// The next unused key.
            fn next_key(&mut self) -> Vec<u8> {
                let key = self.counter.encode();
                self.counter = self.counter.checked_add(1).expect("key sequence exhausted");
                key
            }
        }
    )*};
}
impl_incrementer!(u8, u16, u32, u64);

/// A single typed value stored under a fixed key.
///
/// ```
/// # use massa_sc_sdk::{helpers::{ConstantManager, KeyIncrementer}, mock};
/// # mock::reset();
/// let mut keys = KeyIncrementer::<u8>::new(0);
/// let total: ConstantManager<u64> = ConstantManager::new(&mut keys); // key [0]
/// total.set(&42);
/// assert_eq!(total.must_value(), 42);
/// ```
pub struct ConstantManager<T> {
    key: Vec<u8>,
    _t: PhantomData<T>,
}

impl<T: ArgsEncode + ArgsDecode> ConstantManager<T> {
    /// Take the next key of `seq`.
    pub fn new(seq: &mut impl KeySequence) -> Self {
        Self { key: seq.next_key(), _t: PhantomData }
    }

    /// Use an explicit key.
    pub fn with_key(key: impl Into<Vec<u8>>) -> Self {
        Self { key: key.into(), _t: PhantomData }
    }

    /// The storage key.
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// `true` when a value is stored.
    pub fn exists(&self) -> bool {
        storage::has(&self.key)
    }

    /// Stored value; aborts when absent or invalid.
    pub fn must_value(&self) -> T {
        match self.value() {
            Some(v) => v,
            None => crate::abort("Key not found"),
        }
    }

    /// Stored value, `None` when absent or invalid.
    pub fn value(&self) -> Option<T> {
        storage::try_get(&self.key).and_then(|b| T::decode_bytes(&b).ok())
    }

    /// Store `value`.
    pub fn set(&self, value: &T) {
        storage::set(&self.key, value.encode())
    }

    /// Delete the value; returns whether it existed.
    pub fn delete(&self) -> bool {
        storage::del_if_exists(&self.key)
    }
}

/// A typed map whose entries are stored under `prefix ++ Args(key)`.
pub struct MapManager<K, V> {
    prefix: Vec<u8>,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<K: ArgsEncode, V: ArgsEncode + ArgsDecode> MapManager<K, V> {
    /// Take the next key of `seq` as the map prefix.
    pub fn new(seq: &mut impl KeySequence) -> Self {
        Self::with_prefix(seq.next_key())
    }

    /// Use an explicit key prefix.
    pub fn with_prefix(prefix: impl Into<Vec<u8>>) -> Self {
        Self { prefix: prefix.into(), _k: PhantomData, _v: PhantomData }
    }

    /// The key prefix.
    pub fn prefix(&self) -> &[u8] {
        &self.prefix
    }

    /// Datastore key of `key`: `prefix ++ Args(key)`.
    pub fn storage_key(&self, key: &K) -> Vec<u8> {
        Args::from_bytes(self.prefix.clone()).add(key).into_bytes()
    }

    /// `true` when `key` is in the map.
    pub fn contains(&self, key: &K) -> bool {
        storage::has(self.storage_key(key))
    }

    /// Value for `key`; aborts when absent or invalid.
    pub fn must_value(&self, key: &K) -> V {
        match self.value(key) {
            Some(v) => v,
            None => crate::abort("Key not found"),
        }
    }

    /// Value for `key`, `None` when absent or invalid.
    pub fn value(&self, key: &K) -> Option<V> {
        storage::try_get(self.storage_key(key)).and_then(|b| V::decode_bytes(&b).ok())
    }

    /// Insert or overwrite `key`.
    pub fn set(&self, key: &K, value: &V) {
        storage::set(self.storage_key(key), value.encode())
    }

    /// Delete `key`; returns whether it existed.
    pub fn delete(&self, key: &K) -> bool {
        storage::del_if_exists(self.storage_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;
    use alloc::string::String;

    #[test]
    fn managers() {
        mock::reset();
        let mut seq = KeyIncrementer::<u8>::default();
        let total: ConstantManager<u64> = ConstantManager::new(&mut seq);
        let names: MapManager<u32, String> = MapManager::new(&mut seq);
        assert_eq!(total.key(), &[0]);
        assert_eq!(names.prefix(), &[1]);
        assert!(!total.exists());
        total.set(&9);
        assert_eq!(total.must_value(), 9);
        names.set(&7, &"seven".into());
        assert_eq!(names.storage_key(&7), vec![1, 7, 0, 0, 0]);
        assert_eq!(names.value(&7).as_deref(), Some("seven"));
        assert_eq!(names.value(&8), None);
        assert!(names.delete(&7));
    }
}
