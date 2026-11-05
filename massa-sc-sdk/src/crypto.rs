//! Hashes, signatures and key/address helpers computed by the node.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{env, Address};

/// BLAKE3 hash (Massa's native hash).
#[inline]
pub fn blake3(data: impl AsRef<[u8]>) -> [u8; 32] {
    env::blake3(data.as_ref())
}

/// SHA-256 hash.
#[inline]
pub fn sha256(data: impl AsRef<[u8]>) -> [u8; 32] {
    env::sha256(data.as_ref())
}

/// Keccak-256 hash (Ethereum flavour).
#[inline]
pub fn keccak256(data: impl AsRef<[u8]>) -> [u8; 32] {
    env::keccak256(data.as_ref())
}

/// Verify a Massa signature. `public_key` and `signature` are base58check
/// strings, `data` is the signed message.
#[inline]
pub fn is_signature_valid(public_key: &str, data: &str, signature: &str) -> bool {
    env::is_signature_valid(data, signature, public_key)
}

/// Verify a secp256k1 signature in Ethereum format.
/// * `signature`: 65 bytes `(r, s, v)`, `v` is ignored.
/// * `public_key`: 64-byte uncompressed key without the `0x04` prefix.
#[inline]
pub fn is_evm_signature_valid(data: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    env::is_evm_signature_valid(data, signature, public_key)
}

/// 20-byte Ethereum address of a 64-byte raw secp256k1 public key.
#[inline]
pub fn evm_get_address_from_pubkey(public_key: &[u8]) -> Vec<u8> {
    env::evm_get_address_from_pubkey(public_key)
}

/// Recover the 64-byte secp256k1 public key from a 32-byte `hash` and a
/// 65-byte Ethereum `signature`.
#[inline]
pub fn evm_get_pubkey_from_signature(hash: &[u8], signature: &[u8]) -> Vec<u8> {
    env::evm_get_pubkey_from_signature(hash, signature)
}

/// Address derived from a base58check Massa public key.
#[inline]
pub fn public_key_to_address(public_key: &str) -> Address {
    Address::from(env::public_key_to_address(public_key))
}

/// `true` when `address` is a well-formed Massa address.
#[inline]
pub fn validate_address(address: &str) -> bool {
    env::validate_address(address)
}

/// `true` when `address` is a user account (`AU...`), `false` for a contract.
#[inline]
pub fn is_address_eoa(address: &str) -> bool {
    env::is_address_eoa(address)
}

/// Predictable pseudo-random number. Unsafe for anything of value: block
/// producers can influence it.
#[inline]
pub fn unsafe_random() -> i64 {
    env::unsafe_random()
}

/// Lower-case hexadecimal rendering of bytes (no `0x` prefix).
pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xF) as usize] as char);
    }
    s
}

/// Parse a hexadecimal string (optional `0x` prefix).
pub fn from_hex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex() {
        assert_eq!(to_hex(&[0, 255, 16]), "00ff10");
        assert_eq!(from_hex("0x00ff10").unwrap(), vec![0, 255, 16]);
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
    }

    #[test]
    fn hashes_match_reference_values() {
        crate::mock::reset();
        assert_eq!(
            to_hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            to_hex(&keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
        assert_eq!(to_hex(&blake3(b"")), "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
    }
}
