//! Glue used by the `#[export]` attribute (see `massa-sc-sdk-macros`).
//! Not meant to be used directly.

use alloc::string::String;
use alloc::vec::Vec;

use crate::args::{Args, U256};

/// Types an exported function may take as its single parameter.
pub trait FromParam {
    fn from_param(bytes: Vec<u8>) -> Self;
}

impl FromParam for Vec<u8> {
    #[inline]
    fn from_param(bytes: Vec<u8>) -> Self {
        bytes
    }
}

impl FromParam for Args {
    #[inline]
    fn from_param(bytes: Vec<u8>) -> Self {
        Args::from_bytes(bytes)
    }
}

/// Types an exported function may return. The bytes are handed to the host as
/// the function result (`StaticArray<u8>` on the AssemblyScript side).
pub trait IntoReturn {
    fn into_return_bytes(self) -> Vec<u8>;
}

impl IntoReturn for Vec<u8> {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self
    }
}

impl IntoReturn for &[u8] {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl<const N: usize> IntoReturn for [u8; N] {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl IntoReturn for Args {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}

/// Strings are returned as raw UTF-8 (no length prefix), like an
/// AssemblyScript `stringToBytes(...)` return value.
impl IntoReturn for String {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}

impl IntoReturn for &str {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl IntoReturn for crate::Address {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        self.into_string().into_bytes()
    }
}

macro_rules! impl_into_return_le {
    ($($t:ty),*) => {$(
        impl IntoReturn for $t {
            #[inline]
            fn into_return_bytes(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
        }
    )*};
}
impl_into_return_le!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, f32, f64, U256);

impl IntoReturn for bool {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        alloc::vec![self as u8]
    }
}

/// `Err` aborts the execution with the error's `Display` message.
impl<T: IntoReturn, E: core::fmt::Display> IntoReturn for Result<T, E> {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        match self {
            Ok(v) => v.into_return_bytes(),
            Err(e) => crate::abort(&alloc::format!("{e}")),
        }
    }
}

/// Read the parameter buffer handed by the host.
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn take_param(ptr: i32) -> Vec<u8> {
    crate::memory::take_bytes(ptr)
}

/// Hand a return value to the host.
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn give_return<T: IntoReturn>(value: T) -> i32 {
    crate::memory::leak_bytes(&value.into_return_bytes())
}

/// Unit (e.g. through `Result<(), E>`) returns an empty buffer.
impl IntoReturn for () {
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        Vec::new()
    }
}

impl<T: IntoReturn> IntoReturn for Option<T> {
    /// `None` returns an empty buffer.
    #[inline]
    fn into_return_bytes(self) -> Vec<u8> {
        match self {
            Some(v) => v.into_return_bytes(),
            None => Vec::new(),
        }
    }
}
