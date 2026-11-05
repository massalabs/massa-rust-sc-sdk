//! Operation datastore: the key/value table attached to an `ExecuteSC`
//! operation. It only exists for the duration of that operation and is
//! typically used to pass bytecode and constructor arguments to a deployer
//! script (see the `deployer` example).

use alloc::vec::Vec;

use crate::env;

/// `true` when `key` is present in the operation datastore.
#[inline]
pub fn has_op_key(key: impl AsRef<[u8]>) -> bool {
    env::has_op_key(key.as_ref())
}

/// Value of `key` in the operation datastore; aborts when absent.
#[inline]
pub fn get_op_data(key: impl AsRef<[u8]>) -> Vec<u8> {
    env::get_op_data(key.as_ref())
}

/// Value of `key`, `None` when absent.
#[inline]
pub fn try_get_op_data(key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
    let key = key.as_ref();
    if env::has_op_key(key) {
        Some(env::get_op_data(key))
    } else {
        None
    }
}

/// All keys of the operation datastore.
#[inline]
pub fn get_op_keys() -> Vec<Vec<u8>> {
    env::get_op_keys()
}

/// Keys of the operation datastore starting with `prefix`.
#[inline]
pub fn get_op_keys_prefix(prefix: impl AsRef<[u8]>) -> Vec<Vec<u8>> {
    env::get_op_keys_prefix(prefix.as_ref())
}
