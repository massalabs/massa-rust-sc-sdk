//! Bitmask-based access control (port of the AssemblyScript `AccessControl`).
//! For a role-based model with admin roles, see `massa_sc_standards::access_control`.

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr, Not, Shl};

use crate::args::ArgsEncode;
use crate::{storage, Address};

/// Integer types usable as a permission bitmask.
pub trait Bitmask:
    Copy
    + PartialEq
    + PartialOrd
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Not<Output = Self>
    + Shl<u8, Output = Self>
{
    /// Number of bits, i.e. of distinct permissions.
    const BITS: u8;
    /// No permission.
    const ZERO: Self;
    /// The first permission bit.
    const ONE: Self;
    /// Little-endian storage encoding.
    fn to_bytes(self) -> Vec<u8>;
    /// Decode the storage encoding.
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

macro_rules! impl_bitmask {
    ($($t:ty),*) => {$(
        impl Bitmask for $t {
            const BITS: u8 = <$t>::BITS as u8;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            /// Little-endian storage encoding.
            fn to_bytes(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
            /// Decode the storage encoding.
            fn from_bytes(bytes: &[u8]) -> Option<Self> {
                bytes.try_into().ok().map(<$t>::from_le_bytes)
            }
        }
    )*};
}
impl_bitmask!(u8, u16, u32, u64);

/// Permissions as bits of a `T` bitmask, granted per address and stored under
/// `[module_id] ++ Args(address)`.
///
/// Permissions themselves are not stored: declare them in the same order at
/// every execution (typically as statics at the top of the contract).
///
/// ```
/// # use massa_sc_sdk::{context, mock, security::AccessControl};
/// # mock::reset();
/// let mut acl = AccessControl::<u8>::new(1);
/// let admin = acl.new_permission("admin");
/// let user = acl.new_permission("user");
/// acl.grant_permission(admin, &context::caller());
/// acl.must_have_permission(admin, &context::caller());
/// assert!(!acl.has_permission(user, &context::caller()));
/// ```
pub struct AccessControl<T: Bitmask = u8> {
    module_id: u8,
    count: u8,
    names: Vec<String>,
    _t: core::marker::PhantomData<T>,
}

impl<T: Bitmask> AccessControl<T> {
    /// `module_id` distinguishes the storage keys of several modules.
    pub const fn new(module_id: u8) -> Self {
        Self { module_id, count: 0, names: Vec::new(), _t: core::marker::PhantomData }
    }

    fn storage_key(&self, user: &Address) -> Vec<u8> {
        let mut key = Vec::with_capacity(1 + 4 + user.as_bytes().len());
        key.push(self.module_id);
        user.encode_into(&mut key);
        key
    }

    fn user_access(&self, user: &Address) -> T {
        storage::try_get(self.storage_key(user)).and_then(|b| T::from_bytes(&b)).unwrap_or(T::ZERO)
    }

    fn set_user_access(&self, user: &Address, access: T) {
        storage::set(self.storage_key(user), access.to_bytes())
    }

    fn assert_exists(&self, permission: T) {
        assert!(permission != T::ZERO && permission < (T::ONE << self.count), "Permission does not exist.");
    }

    fn name(&self, permission: T) -> &str {
        let mut idx = 0u8;
        while idx < self.count && (T::ONE << idx) != permission {
            idx += 1;
        }
        self.names.get(idx as usize).map(String::as_str).unwrap_or("?")
    }

    /// Declare a new permission; returns its bit.
    pub fn new_permission(&mut self, name: &str) -> T {
        assert!(self.count < T::BITS, "Maximum number of permissions reached.");
        self.names.push(String::from(name));
        self.count += 1;
        T::ONE << (self.count - 1)
    }

    /// Grant `permission` to `user`; aborts when already granted.
    pub fn grant_permission(&self, permission: T, user: &Address) {
        self.assert_exists(permission);
        let access = self.user_access(user);
        assert!(
            (access & permission) != permission,
            "User already has '{}' permission.",
            self.name(permission)
        );
        self.set_user_access(user, access | permission);
    }

    /// Revoke `permission` from `user`; aborts when not granted.
    pub fn revoke_permission(&self, permission: T, user: &Address) {
        self.assert_exists(permission);
        let access = self.user_access(user);
        assert!(
            (access & permission) == permission,
            "User does not have '{}' permission.",
            self.name(permission)
        );
        self.set_user_access(user, access & !permission);
    }

    /// `true` when `user` has every bit of `permission`.
    pub fn has_permission(&self, permission: T, user: &Address) -> bool {
        self.assert_exists(permission);
        (self.user_access(user) & permission) == permission
    }

    /// Abort unless `user` has `permission`.
    pub fn must_have_permission(&self, permission: T, user: &Address) {
        assert!(
            self.has_permission(permission, user),
            "User does not have '{}' permission.",
            self.name(permission)
        );
    }

    /// `true` when `user` has at least one bit of `permissions`.
    pub fn has_any_permission(&self, permissions: T, user: &Address) -> bool {
        assert!(permissions != T::ZERO && permissions < (T::ONE << self.count), "Permission does not exist.");
        (self.user_access(user) & permissions) != T::ZERO
    }

    /// Abort unless `user` has at least one bit of `permissions`.
    pub fn must_have_any_permission(&self, permissions: T, user: &Address) {
        assert!(self.has_any_permission(permissions, user), "User does not have any of the permissions.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn permissions() {
        mock::reset();
        let mut acl = AccessControl::<u8>::new(1);
        let admin = acl.new_permission("admin");
        let user = acl.new_permission("user");
        let a = Address::from("AU1a");
        assert!(!acl.has_permission(admin, &a));
        acl.grant_permission(admin, &a);
        assert!(acl.has_permission(admin, &a));
        assert!(acl.has_any_permission(admin | user, &a));
        assert!(!acl.has_permission(admin | user, &a));
        acl.revoke_permission(admin, &a);
        assert!(!acl.has_permission(admin, &a));
    }

    #[test]
    #[should_panic(expected = "User already has 'admin' permission.")]
    fn double_grant() {
        mock::reset();
        let mut acl = AccessControl::<u8>::new(1);
        let admin = acl.new_permission("admin");
        let a = Address::from("AU1a");
        acl.grant_permission(admin, &a);
        acl.grant_permission(admin, &a);
    }
}
