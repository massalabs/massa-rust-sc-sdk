//! Ownable: a single owner address stored under the `OWNER` key
//! (storage layout identical to `@massalabs/sc-standards` `ownership`).
//!
//! Entry points (export them with [`crate::export_ownable!`]):
//! `setOwner(address)`, `ownerAddress() -> string bytes`, `isOwner(address) -> bool`.

use massa_sc_sdk::prelude::*;

/// Datastore key holding the owner address (raw string).
pub const OWNER_KEY: &str = "OWNER";
/// Event emitted when the owner changes: `CHANGE_OWNER:<address>`.
pub const CHANGE_OWNER_EVENT_NAME: &str = "CHANGE_OWNER";

/// Current owner, `None` when not set.
pub fn owner() -> Option<Address> {
    storage::try_get(OWNER_KEY).map(|b| Address::from(String::from_utf8_lossy(&b).into_owned()))
}

/// `true` when `account` is the owner.
pub fn is_owner(account: &Address) -> bool {
    owner().is_some_and(|o| &o == account)
}

/// Abort unless the caller is the owner (`Caller is not the owner`).
pub fn only_owner() {
    assert!(storage::has(OWNER_KEY), "Owner is not set");
    assert!(is_owner(&context::caller()), "Caller is not the owner");
}

/// Set the owner. When an owner already exists, only it may call this.
pub fn set_owner(new_owner: &Address) {
    if storage::has(OWNER_KEY) {
        only_owner();
    }
    storage::set(OWNER_KEY, new_owner.as_bytes());
    generate_event(&create_event(CHANGE_OWNER_EVENT_NAME, &[new_owner.as_str()]));
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `setOwner(newOwner: string)`
    pub fn set_owner(mut args: Args) {
        let new_owner: Address = args.must_next("newOwnerAddress argument is missing or invalid");
        super::set_owner(&new_owner);
    }

    /// `ownerAddress() -> string bytes` (empty when not set).
    pub fn owner_address() -> Vec<u8> {
        storage::try_get(OWNER_KEY).unwrap_or_default()
    }

    /// `isOwner(address: string) -> bool`
    pub fn is_owner(mut args: Args) -> bool {
        if !storage::has(OWNER_KEY) {
            return false;
        }
        let account: Address = args.must_next("address argument is missing or invalid");
        super::is_owner(&account)
    }
}

/// Export the Ownable entry points (`setOwner`, `ownerAddress`, `isOwner`).
#[macro_export]
macro_rules! export_ownable {
    () => {
        #[::massa_sc_sdk::export(name = "setOwner")]
        pub fn __std_set_owner(args: ::massa_sc_sdk::Args) {
            $crate::ownership::entrypoints::set_owner(args)
        }
        #[::massa_sc_sdk::export(name = "ownerAddress")]
        pub fn __std_owner_address() -> ::massa_sc_sdk::prelude::Vec<u8> {
            $crate::ownership::entrypoints::owner_address()
        }
        #[::massa_sc_sdk::export(name = "isOwner")]
        pub fn __std_is_owner(args: ::massa_sc_sdk::Args) -> bool {
            $crate::ownership::entrypoints::is_owner(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn ownership_flow() {
        mock::reset();
        let caller = context::caller();
        assert_eq!(owner(), None);
        assert!(!entrypoints::is_owner(Args::new().add(&caller)));
        set_owner(&caller);
        assert_eq!(owner(), Some(caller.clone()));
        assert!(is_owner(&caller));
        assert_eq!(mock::events(), vec![format!("CHANGE_OWNER:{caller}")]);
        only_owner();
        let other = mock::new_user_address();
        entrypoints::set_owner(Args::new().add(&other));
        assert_eq!(entrypoints::owner_address(), other.as_bytes());
    }

    #[test]
    #[should_panic(expected = "Caller is not the owner")]
    fn only_owner_rejects_others() {
        mock::reset();
        set_owner(&mock::new_user_address());
        only_owner();
    }
}
