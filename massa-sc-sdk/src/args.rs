//! Argument serialization, byte-compatible with `@massalabs/as-types` `Args`
//! and `@massalabs/massa-web3` `Args`:
//!
//! | type                       | encoding                                        |
//! |----------------------------|-------------------------------------------------|
//! | `bool`                     | 1 byte (0/1)                                    |
//! | `u8`..`u64`, `i8`..`i64`   | little-endian, natural width                    |
//! | `u128`, `i128`, `U256`     | little-endian, 16 / 16 / 32 bytes               |
//! | `f32`, `f64`               | IEEE-754 little-endian                          |
//! | `str` / `String`           | `u32` byte length + UTF-8                       |
//! | `[T]` / `Vec<T>` / `[T;N]` | `u32` byte length + concatenated elements       |
//! | `Address`                  | as a string                                     |
//! | `T: Serializable`          | concatenated fields, no length prefix           |
//!
//! Build arguments with [`Args::new`] / [`Args::add`], read them with
//! [`Args::next`] or the typed `next_*` helpers.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub use ethnum::{I256, U256};

/// Deserialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsError {
    /// Not enough bytes left to read a value of the named type.
    OutOfRange(&'static str),
    /// A string field is not valid UTF-8.
    InvalidUtf8,
    /// The data is malformed (e.g. an array whose length is not a multiple
    /// of its element size).
    Malformed(&'static str),
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgsError::OutOfRange(t) => {
                write!(f, "can't deserialize {t} from given argument: out of range")
            }
            ArgsError::InvalidUtf8 => f.write_str("can't deserialize string: invalid UTF-8"),
            ArgsError::Malformed(t) => write!(f, "can't deserialize {t}: malformed data"),
        }
    }
}

/// Result of a deserialization.
pub type Result<T> = core::result::Result<T, ArgsError>;

/// Types that can be appended to [`Args`].
pub trait ArgsEncode {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>);

    /// Standalone encoding (same bytes as `Args::new().add(self)`).
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_into(&mut out);
        out
    }
}

/// Types that can be read from [`Args`].
pub trait ArgsDecode: Sized {
    /// Read a value at the current offset of `args`, advancing it.
    fn decode(args: &mut Args) -> Result<Self>;

    /// Decode a standalone value from `bytes` (ignores trailing data).
    fn decode_bytes(bytes: &[u8]) -> Result<Self> {
        Self::decode(&mut Args::from(bytes))
    }
}

/// Anything that is both [`ArgsEncode`] and [`ArgsDecode`]: the Rust
/// counterpart of the AssemblyScript `Serializable` interface. User structures
/// are serialized as the concatenation of their fields, in declaration order,
/// with `#[derive(Serializable)]`:
///
/// ```
/// # use massa_sc_sdk::prelude::*;
/// #[derive(Serializable, PartialEq, Debug)]
/// struct Order { owner: Address, amount: u64, memo: String }
///
/// let order = Order { owner: Address::from("AU1..."), amount: 5, memo: "hi".into() };
/// let bytes = Args::new().add(&order).into_bytes();   // or order.serialize()
/// let back: Order = Args::from(bytes).next()?;        // or Order::deserialize(&bytes)?
/// assert_eq!(back, order);
/// # Ok::<(), ArgsError>(())
/// ```
pub trait Serializable: ArgsEncode + ArgsDecode {
    /// Serialized bytes (the [`Args`] encoding).
    fn serialize(&self) -> Vec<u8> {
        self.encode()
    }
    /// Decode from serialized bytes.
    fn deserialize(bytes: &[u8]) -> Result<Self> {
        Self::decode_bytes(bytes)
    }
}

impl<T: ArgsEncode + ArgsDecode> Serializable for T {}

/// Serialized argument list (builder and reader).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Args {
    data: Vec<u8>,
    offset: usize,
}

impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Args({:?}, offset={})", self.data, self.offset)
    }
}

#[allow(clippy::should_implement_trait)]
impl Args {
    /// Empty argument list.
    pub const fn new() -> Self {
        Self { data: Vec::new(), offset: 0 }
    }

    /// Reader over existing serialized data.
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data, offset: 0 }
    }

    /// Reader starting at `offset`.
    pub fn with_offset(data: Vec<u8>, offset: usize) -> Self {
        Self { data, offset }
    }

    /// Append a value (builder style).
    pub fn add<T: ArgsEncode>(mut self, value: T) -> Self {
        value.encode_into(&mut self.data);
        self
    }

    /// Append a value in place.
    pub fn push<T: ArgsEncode>(&mut self, value: T) -> &mut Self {
        value.encode_into(&mut self.data);
        self
    }

    /// Read the next value.
    pub fn next<T: ArgsDecode>(&mut self) -> Result<T> {
        T::decode(self)
    }

    /// Read the next value, aborting the execution with `what` when missing or
    /// invalid (`"Can't deserialize <what>."`).
    pub fn must_next<T: ArgsDecode>(&mut self, what: &str) -> T {
        match T::decode(self) {
            Ok(v) => v,
            Err(_) => crate::abort(&crate::concat_str(&["Can't deserialize ", what, "."])),
        }
    }

    /// Read the next `bool`.
    pub fn next_bool(&mut self) -> Result<bool> {
        self.next()
    }
    /// Read the next `u8`.
    pub fn next_u8(&mut self) -> Result<u8> {
        self.next()
    }
    /// Read the next `i8`.
    pub fn next_i8(&mut self) -> Result<i8> {
        self.next()
    }
    /// Read the next `u16`.
    pub fn next_u16(&mut self) -> Result<u16> {
        self.next()
    }
    /// Read the next `i16`.
    pub fn next_i16(&mut self) -> Result<i16> {
        self.next()
    }
    /// Read the next `u32`.
    pub fn next_u32(&mut self) -> Result<u32> {
        self.next()
    }
    /// Read the next `i32`.
    pub fn next_i32(&mut self) -> Result<i32> {
        self.next()
    }
    /// Read the next `u64`.
    pub fn next_u64(&mut self) -> Result<u64> {
        self.next()
    }
    /// Read the next `i64`.
    pub fn next_i64(&mut self) -> Result<i64> {
        self.next()
    }
    /// Read the next `u128`.
    pub fn next_u128(&mut self) -> Result<u128> {
        self.next()
    }
    /// Read the next `i128`.
    pub fn next_i128(&mut self) -> Result<i128> {
        self.next()
    }
    /// Read the next `U256` (32 bytes, little-endian).
    pub fn next_u256(&mut self) -> Result<U256> {
        self.next()
    }
    /// Read the next `f32`.
    pub fn next_f32(&mut self) -> Result<f32> {
        self.next()
    }
    /// Read the next `f64`.
    pub fn next_f64(&mut self) -> Result<f64> {
        self.next()
    }
    /// Read the next string (`u32` length + UTF-8).
    pub fn next_string(&mut self) -> Result<String> {
        self.next()
    }
    /// Read the next byte string (`u32` length + bytes).
    pub fn next_bytes(&mut self) -> Result<Vec<u8>> {
        self.next()
    }
    /// Read the next [`Address`](crate::Address) (encoded as a string).
    pub fn next_address(&mut self) -> Result<crate::Address> {
        self.next()
    }
    /// Read the next [`Serializable`] structure.
    pub fn next_serializable<T: Serializable>(&mut self) -> Result<T> {
        T::decode(self)
    }
    /// Read the next array (`u32` byte length + elements): fixed-size numbers, `U256`, strings, structures...
    pub fn next_array<T: ArgsDecode>(&mut self) -> Result<Vec<T>> {
        self.next()
    }
    /// Read the next array of strings.
    pub fn next_string_array(&mut self) -> Result<Vec<String>> {
        self.next()
    }

    /// Serialized bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Copy of the serialized bytes (the AssemblyScript `args.serialize()`).
    /// To nest an `Args` into another one, add these bytes:
    /// `outer.add(inner.serialize())`, read back with `next_bytes()`.
    pub fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Consume into the serialized bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Number of serialized bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// `true` when there are no serialized bytes.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Current read position.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Bytes not read yet.
    pub fn remaining(&self) -> &[u8] {
        &self.data[self.offset.min(self.data.len())..]
    }

    /// `true` once every byte has been read.
    pub fn is_exhausted(&self) -> bool {
        self.offset >= self.data.len()
    }

    #[inline]
    fn take(&mut self, n: usize, what: &'static str) -> Result<&[u8]> {
        let end = self.offset.checked_add(n).ok_or(ArgsError::OutOfRange(what))?;
        if end > self.data.len() {
            return Err(ArgsError::OutOfRange(what));
        }
        let s = &self.data[self.offset..end];
        self.offset = end;
        Ok(s)
    }

    #[inline]
    fn take_len_prefixed(&mut self, what: &'static str) -> Result<&[u8]> {
        let len = u32::decode_at(self, what)? as usize;
        self.take(len, what)
    }
}

impl From<Vec<u8>> for Args {
    fn from(data: Vec<u8>) -> Self {
        Self::from_bytes(data)
    }
}

impl From<&[u8]> for Args {
    fn from(data: &[u8]) -> Self {
        Self::from_bytes(data.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Args {
    fn from(data: &[u8; N]) -> Self {
        Self::from_bytes(data.to_vec())
    }
}

impl From<Args> for Vec<u8> {
    fn from(a: Args) -> Self {
        a.data
    }
}

impl AsRef<[u8]> for Args {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

// ---------------------------------------------------------------------------
// primitives
// ---------------------------------------------------------------------------

trait FixedDecode: Sized {
    fn decode_at(args: &mut Args, what: &'static str) -> Result<Self>;
}

macro_rules! impl_int {
    ($($t:ty => $name:literal),* $(,)?) => {$(
        impl ArgsEncode for $t {
            /// Append the encoding of `self` to `out`.
            #[inline]
            fn encode_into(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        }
        impl FixedDecode for $t {
            #[inline]
            fn decode_at(args: &mut Args, what: &'static str) -> Result<Self> {
                let s = args.take(core::mem::size_of::<$t>(), what)?;
                let mut buf = [0u8; core::mem::size_of::<$t>()];
                buf.copy_from_slice(s);
                Ok(<$t>::from_le_bytes(buf))
            }
        }
        impl ArgsDecode for $t {
            #[inline]
            fn decode(args: &mut Args) -> Result<Self> {
                Self::decode_at(args, $name)
            }
        }
    )*};
}

impl_int!(
    u8 => "u8", i8 => "i8", u16 => "u16", i16 => "i16", u32 => "u32", i32 => "i32",
    u64 => "u64", i64 => "i64", u128 => "u128", i128 => "i128", f32 => "f32", f64 => "f64",
);

impl ArgsEncode for U256 {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

impl ArgsDecode for U256 {
    fn decode(args: &mut Args) -> Result<Self> {
        let s = args.take(32, "u256")?;
        let mut buf = [0u8; 32];
        buf.copy_from_slice(s);
        Ok(U256::from_le_bytes(buf))
    }
}

impl ArgsEncode for I256 {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

impl ArgsDecode for I256 {
    fn decode(args: &mut Args) -> Result<Self> {
        let s = args.take(32, "i256")?;
        let mut buf = [0u8; 32];
        buf.copy_from_slice(s);
        Ok(I256::from_le_bytes(buf))
    }
}

impl ArgsEncode for bool {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    }
}

impl ArgsDecode for bool {
    fn decode(args: &mut Args) -> Result<Self> {
        Ok(args.take(1, "bool")?[0] != 0)
    }
}

// ---------------------------------------------------------------------------
// strings & byte strings
// ---------------------------------------------------------------------------

impl ArgsEncode for str {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        (self.len() as u32).encode_into(out);
        out.extend_from_slice(self.as_bytes());
    }
}

impl ArgsEncode for String {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.as_str().encode_into(out)
    }
}

impl ArgsDecode for String {
    fn decode(args: &mut Args) -> Result<Self> {
        let s = args.take_len_prefixed("string")?;
        core::str::from_utf8(s).map(String::from).map_err(|_| ArgsError::InvalidUtf8)
    }
}

impl<T: ArgsEncode> ArgsEncode for [T] {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        let len_pos = out.len();
        out.extend_from_slice(&[0, 0, 0, 0]);
        for item in self {
            item.encode_into(out);
        }
        let byte_len = (out.len() - len_pos - 4) as u32;
        out[len_pos..len_pos + 4].copy_from_slice(&byte_len.to_le_bytes());
    }
}

impl<T: ArgsEncode> ArgsEncode for Vec<T> {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.as_slice().encode_into(out)
    }
}

impl<T: ArgsEncode, const N: usize> ArgsEncode for [T; N] {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.as_slice().encode_into(out)
    }
}

impl<T: ArgsDecode> ArgsDecode for Vec<T> {
    fn decode(args: &mut Args) -> Result<Self> {
        let byte_len = u32::decode_at(args, "array")? as usize;
        let end = args
            .offset
            .checked_add(byte_len)
            .filter(|&e| e <= args.data.len())
            .ok_or(ArgsError::OutOfRange("array"))?;
        let mut items = Vec::new();
        while args.offset < end {
            items.push(T::decode(args)?);
        }
        if args.offset != end {
            return Err(ArgsError::Malformed("array"));
        }
        Ok(items)
    }
}

impl<T: ArgsDecode + Default + Copy, const N: usize> ArgsDecode for [T; N] {
    fn decode(args: &mut Args) -> Result<Self> {
        let v: Vec<T> = Vec::decode(args)?;
        if v.len() != N {
            return Err(ArgsError::Malformed("fixed-size array"));
        }
        let mut out = [T::default(); N];
        out.copy_from_slice(&v);
        Ok(out)
    }
}

impl<T: ArgsEncode + ?Sized> ArgsEncode for &T {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        (**self).encode_into(out)
    }
}

impl<T: ArgsEncode + ?Sized> ArgsEncode for &mut T {
    /// Append the encoding of `self` to `out`.
    fn encode_into(&self, out: &mut Vec<u8>) {
        (**self).encode_into(out)
    }
}

impl<T: ArgsEncode> ArgsEncode for Option<T> {
    /// `Option` is encoded as a `bool` flag followed by the value when `Some`,
    /// i.e. what an AssemblyScript or TypeScript side writes as
    /// `add(present).add(value)` and reads as `nextBool()` then the value.
    /// `as-types` has no optional type of its own.
    fn encode_into(&self, out: &mut Vec<u8>) {
        match self {
            Some(v) => {
                out.push(1);
                v.encode_into(out);
            }
            None => out.push(0),
        }
    }
}

impl<T: ArgsDecode> ArgsDecode for Option<T> {
    fn decode(args: &mut Args) -> Result<Self> {
        if bool::decode(args)? {
            Ok(Some(T::decode(args)?))
        } else {
            Ok(None)
        }
    }
}

impl ArgsEncode for () {
    fn encode_into(&self, _out: &mut Vec<u8>) {}
}

impl ArgsDecode for () {
    fn decode(_args: &mut Args) -> Result<Self> {
        Ok(())
    }
}

macro_rules! impl_tuple {
    ($($name:ident),+) => {
        impl<$($name: ArgsEncode),+> ArgsEncode for ($($name,)+) {
            /// Append the encoding of `self` to `out`.
            #[allow(non_snake_case)]
            fn encode_into(&self, out: &mut Vec<u8>) {
                let ($($name,)+) = self;
                $($name.encode_into(out);)+
            }
        }
        impl<$($name: ArgsDecode),+> ArgsDecode for ($($name,)+) {
            fn decode(args: &mut Args) -> Result<Self> {
                Ok(($($name::decode(args)?,)+))
            }
        }
    };
}
impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Address;

    #[test]
    fn primitives_roundtrip_and_layout() {
        let a = Args::new()
            .add(true)
            .add(7u8)
            .add(-2i16)
            .add(0xDEADBEEFu32)
            .add(u64::MAX)
            .add(1.5f32)
            .add(-2.25f64)
            .add(u128::MAX - 1)
            .add(U256::from(42u64));
        let bytes = a.as_bytes().to_vec();
        assert_eq!(bytes.len(), 1 + 1 + 2 + 4 + 8 + 4 + 8 + 16 + 32);
        assert_eq!(&bytes[..4], &[1, 7, 0xFE, 0xFF]);
        assert_eq!(&bytes[4..8], &[0xEF, 0xBE, 0xAD, 0xDE]);
        let mut r = Args::from(bytes);
        assert!(r.next_bool().unwrap());
        assert_eq!(r.next_u8().unwrap(), 7);
        assert_eq!(r.next_i16().unwrap(), -2);
        assert_eq!(r.next_u32().unwrap(), 0xDEADBEEF);
        assert_eq!(r.next_u64().unwrap(), u64::MAX);
        assert_eq!(r.next_f32().unwrap(), 1.5);
        assert_eq!(r.next_f64().unwrap(), -2.25);
        assert_eq!(r.next_u128().unwrap(), u128::MAX - 1);
        assert_eq!(r.next_u256().unwrap(), U256::from(42u64));
        assert!(r.is_exhausted());
        assert_eq!(r.next_u8(), Err(ArgsError::OutOfRange("u8")));
    }

    #[test]
    fn strings_bytes_arrays() {
        let bytes = Args::new()
            .add("héllo")
            .add(vec![1u8, 2, 3])
            .add(&[9u8, 8][..])
            .add(vec!["a".to_string(), "bc".to_string()])
            .add(vec![1u32, 2, 3])
            .add(Vec::<u64>::new())
            .into_bytes();
        // "héllo" is 6 bytes in UTF-8.
        assert_eq!(&bytes[..10], &[6, 0, 0, 0, b'h', 0xC3, 0xA9, b'l', b'l', b'o']);
        assert_eq!(&bytes[10..17], &[3, 0, 0, 0, 1, 2, 3]);
        let mut r = Args::from(bytes);
        assert_eq!(r.next_string().unwrap(), "héllo");
        assert_eq!(r.next_bytes().unwrap(), vec![1, 2, 3]);
        assert_eq!(r.next::<[u8; 2]>().unwrap(), [9, 8]);
        assert_eq!(r.next_string_array().unwrap(), vec!["a", "bc"]);
        assert_eq!(r.next_array::<u32>().unwrap(), vec![1, 2, 3]);
        assert_eq!(r.next_array::<u64>().unwrap(), Vec::<u64>::new());
        assert!(r.is_exhausted());
    }

    // Vectors produced by @massalabs/as-types / massa-web3.
    #[test]
    fn as_types_compat_vectors() {
        // new Args().add("hello").add(u64(42)).add(true)
        let expected = [5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o', 42, 0, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(Args::new().add("hello").add(42u64).add(true).as_bytes(), &expected);
        // string array ["ab","c"]: total len 4+2+4+1 = 11
        let expected = [11, 0, 0, 0, 2, 0, 0, 0, b'a', b'b', 1, 0, 0, 0, b'c'];
        assert_eq!(Args::new().add(["ab", "c"]).as_bytes(), &expected);
        // Array<i32> [1,-1]
        let expected = [8, 0, 0, 0, 1, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(Args::new().add(vec![1i32, -1]).as_bytes(), &expected);
    }

    struct Pair {
        key: String,
        value: u64,
    }
    impl ArgsEncode for Pair {
        /// Append the encoding of `self` to `out`.
        fn encode_into(&self, out: &mut Vec<u8>) {
            self.key.encode_into(out);
            self.value.encode_into(out);
        }
    }
    impl ArgsDecode for Pair {
        fn decode(args: &mut Args) -> Result<Self> {
            Ok(Pair { key: args.next()?, value: args.next()? })
        }
    }

    #[test]
    fn serializable_and_arrays_of_serializable() {
        let p = Pair { key: "k".into(), value: 5 };
        let bytes = Args::new().add(&p).add(vec![&p, &p]).into_bytes();
        // struct: 4+1+8 = 13 bytes, then array: 4 + 26
        assert_eq!(bytes.len(), 13 + 4 + 26);
        let mut r = Args::from(bytes);
        let back: Pair = r.next().unwrap();
        assert_eq!(back.key, "k");
        assert_eq!(back.value, 5);
        let arr: Vec<Pair> = r.next().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1].value, 5);
        assert_eq!(p.serialize(), Args::new().add(&p).into_bytes());
    }

    #[test]
    fn address_option_tuple_nested_args() {
        let addr = Address::from("AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq");
        let inner = Args::new().add(1u8);
        // Nesting is explicit, like `add(inner.serialize())` in AssemblyScript.
        let bytes = Args::new()
            .add(&addr)
            .add(Some(3u16))
            .add(None::<u16>)
            .add((1u8, "x"))
            .add(inner.serialize())
            .into_bytes();
        let mut r = Args::from(bytes);
        assert_eq!(r.next_address().unwrap(), addr);
        assert_eq!(r.next::<Option<u16>>().unwrap(), Some(3));
        assert_eq!(r.next::<Option<u16>>().unwrap(), None);
        assert_eq!(r.next::<(u8, String)>().unwrap(), (1, "x".to_string()));
        assert_eq!(Args::from(r.next_bytes().unwrap()), inner);
        assert!(r.is_exhausted());
    }

    #[test]
    fn malformed_arrays_are_rejected() {
        // byte length 3 for u32 elements
        let mut r = Args::from(vec![3u8, 0, 0, 0, 1, 2, 3]);
        assert_eq!(r.next_array::<u32>(), Err(ArgsError::OutOfRange("u32")));
        let mut r = Args::from(vec![9u8, 0, 0, 0, 1]);
        assert_eq!(r.next_bytes(), Err(ArgsError::OutOfRange("array")));
        let mut r = Args::from(vec![1u8, 0, 0, 0, 0xFF]);
        assert_eq!(r.next_string(), Err(ArgsError::InvalidUtf8));
    }
}
