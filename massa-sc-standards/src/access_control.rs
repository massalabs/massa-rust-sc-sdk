//! Role-based access control on top of [`crate::ownership`]: the owner grants
//! and revokes named roles; members of a role can renounce it. Members of each
//! role are stored as a string array under `_ROLES<role>` (same layout as
//! `@massalabs/sc-standards` `accessControl`).
//!
//! Entry points (export with [`crate::export_access_control!`]):
//! `grantRole(role, account)`, `revokeRole(role, account)`,
//! `hasRole(role, account) -> bool`, `members(role) -> Args(string[])`,
//! `onlyRole(role)`.

use massa_sc_sdk::prelude::*;

use crate::ownership;

/// Prefix of the datastore keys holding the members of each role.
pub const ROLES_KEY: &str = "_ROLES";
/// Event emitted when a role is granted: `ROLE_GRANTED_EVENT:<role>,<account>`.
pub const ROLE_GRANTED_EVENT: &str = "ROLE_GRANTED_EVENT";
/// Event emitted when a role is revoked: `ROLE_REVOKED_EVENT:<role>,<account>`.
pub const ROLE_REVOKED_EVENT: &str = "ROLE_REVOKED_EVENT";

fn role_key(role: &str) -> Vec<u8> {
    let mut k = Vec::with_capacity(ROLES_KEY.len() + role.len());
    k.extend_from_slice(ROLES_KEY.as_bytes());
    k.extend_from_slice(role.as_bytes());
    k
}

/// Accounts holding `role`.
pub fn members(role: &str) -> Vec<String> {
    storage::try_get(role_key(role)).and_then(|b| Args::from(b).next_string_array().ok()).unwrap_or_default()
}

fn set_members(role: &str, members: &[String]) {
    storage::set(role_key(role), Args::new().add(members));
}

/// `true` when `account` holds `role`.
pub fn has_role(role: &str, account: &Address) -> bool {
    members(role).iter().any(|m| m == account.as_str())
}

/// Abort unless the caller holds `role`.
pub fn only_role(role: &str) {
    assert!(has_role(role, &context::caller()), "Caller does not have {role} role");
}

/// Grant `role` to `account`. Only the owner may call this.
pub fn grant_role(role: &str, account: &Address) {
    ownership::only_owner();
    assert!(!has_role(role, account), "Account already has {role} role");
    let mut list = members(role);
    list.push(account.to_string());
    set_members(role, &list);
    generate_event(&create_event(ROLE_GRANTED_EVENT, &[role, account.as_str()]));
}

/// Revoke `role` from `account`. The owner may revoke anyone; an account may
/// renounce its own role.
pub fn revoke_role(role: &str, account: &Address) {
    let caller = context::caller();
    if !ownership::is_owner(&caller) {
        assert!(&caller == account, "Caller is not {account} or admin");
        assert!(has_role(role, account), "Account does not have {role} role");
    }
    let mut list = members(role);
    if let Some(pos) = list.iter().position(|m| m == account.as_str()) {
        list.swap_remove(pos);
    }
    set_members(role, &list);
    generate_event(&create_event(ROLE_REVOKED_EVENT, &[role, account.as_str()]));
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `grantRole(role: string, account: string)`
    pub fn grant_role(mut args: Args) {
        let initial = coins::balance();
        let role: String = args.must_next("role argument is missing or invalid");
        let account: Address = args.must_next("account argument is missing or invalid");
        super::grant_role(&role, &account);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `revokeRole(role: string, account: string)`
    pub fn revoke_role(mut args: Args) {
        let initial = coins::balance();
        let role: String = args.must_next("role argument is missing or invalid");
        let account: Address = args.must_next("account argument is missing or invalid");
        super::revoke_role(&role, &account);
        coins::transfer_remaining(initial, 0, None);
    }

    /// `hasRole(role: string, account: string) -> bool`
    pub fn has_role(mut args: Args) -> bool {
        let role: String = args.must_next("role argument is missing or invalid");
        let account: Address = args.must_next("account argument is missing or invalid");
        super::has_role(&role, &account)
    }

    /// `members(role: string) -> Args(string[])`
    pub fn members(mut args: Args) -> Args {
        let role: String = args.must_next("role argument is missing or invalid");
        Args::new().add(super::members(&role))
    }

    /// `onlyRole(role: string)`
    pub fn only_role(mut args: Args) {
        let role: String = args.must_next("role argument is missing or invalid");
        super::only_role(&role);
    }
}

/// Export the role-based access control entry points
/// (`grantRole`, `revokeRole`, `hasRole`, `members`, `onlyRole`).
#[macro_export]
macro_rules! export_access_control {
    () => {
        #[::massa_sc_sdk::export(name = "grantRole")]
        pub fn __std_grant_role(args: ::massa_sc_sdk::Args) {
            $crate::access_control::entrypoints::grant_role(args)
        }
        #[::massa_sc_sdk::export(name = "revokeRole")]
        pub fn __std_revoke_role(args: ::massa_sc_sdk::Args) {
            $crate::access_control::entrypoints::revoke_role(args)
        }
        #[::massa_sc_sdk::export(name = "hasRole")]
        pub fn __std_has_role(args: ::massa_sc_sdk::Args) -> bool {
            $crate::access_control::entrypoints::has_role(args)
        }
        #[::massa_sc_sdk::export(name = "members")]
        pub fn __std_members(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::Args {
            $crate::access_control::entrypoints::members(args)
        }
        #[::massa_sc_sdk::export(name = "onlyRole")]
        pub fn __std_only_role(args: ::massa_sc_sdk::Args) {
            $crate::access_control::entrypoints::only_role(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn roles() {
        mock::reset();
        let owner = context::caller();
        ownership::set_owner(&owner);
        let alice = mock::new_user_address();
        assert!(!has_role("minter", &alice));
        grant_role("minter", &alice);
        assert!(has_role("minter", &alice));
        assert_eq!(members("minter"), vec![alice.to_string()]);
        // alice renounces her own role
        mock::set_caller(&alice);
        only_role("minter");
        revoke_role("minter", &alice);
        assert!(members("minter").is_empty());
    }

    #[test]
    #[should_panic(expected = "Caller is not the owner")]
    fn only_owner_grants() {
        mock::reset();
        ownership::set_owner(&mock::new_user_address());
        grant_role("minter", &context::caller());
    }
}
