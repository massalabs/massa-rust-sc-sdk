//! Massa addresses (`AU...` user accounts, `AS...` smart contracts).

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::args::{Args, ArgsDecode, ArgsEncode, Result as ArgsResult};

/// A Massa address, stored as its base58check string form.
///
/// It is a thin wrapper: construction never validates (use
/// [`Address::is_valid`] / [`Address::assert_valid`] when the value comes from
/// an untrusted argument). Serialized in [`Args`] exactly like a string.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Address(String);

impl Address {
    pub const fn new(address: String) -> Self {
        Self(address)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    /// Raw bytes of the string form (useful as a storage key suffix).
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Asks the node whether the address is well formed.
    pub fn is_valid(&self) -> bool {
        crate::env::validate_address(&self.0)
    }

    /// Aborts the execution when the address is not valid.
    pub fn assert_valid(&self) -> &Self {
        if !self.is_valid() {
            crate::abort(&alloc::format!("{} is not a valid address", self.0));
        }
        self
    }

    /// `true` for a valid smart-contract address (`AS...`).
    pub fn is_smart_contract(&self) -> bool {
        self.0.starts_with("AS") && self.is_valid()
    }

    /// `true` for a valid externally-owned (user) address (`AU...`).
    pub fn is_eoa(&self) -> bool {
        self.0.starts_with("AU") && self.is_valid()
    }

    /// Balance of this address in nanoMAS.
    pub fn balance(&self) -> u64 {
        crate::env::balance_of(&self.0)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.0)
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Address {
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl From<&String> for Address {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<Address> for String {
    fn from(a: Address) -> Self {
        a.0
    }
}

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Address {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Address {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Address {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl ArgsEncode for Address {
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.0.as_str().encode_into(out)
    }
}

impl ArgsDecode for Address {
    fn decode(args: &mut Args) -> ArgsResult<Self> {
        Ok(Self(String::decode(args)?))
    }
}
