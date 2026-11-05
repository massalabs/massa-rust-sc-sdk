//! C-Layer specific ABIs.
//!
//! [C-Layer](https://github.com/massalabs/c-layer) is a Massa fork whose
//! runtime exposes a few extra host functions. A module that imports one of
//! them cannot be instantiated on a plain Massa node (missing import), so
//! contracts using this module only run on C-Layer networks
//! (see [`crate::context::is_clayer`] and the `CLAYER_*_CHAIN_ID` constants).
//!
//! Imports are only emitted for functions that are actually used: merely
//! depending on this SDK never makes a contract C-Layer specific.

use crate::env;

/// MiMC hash over the BN254 scalar field (gnark-crypto compatible: 110 rounds,
/// exponent 5, Miyaguchi-Preneel construction).
///
/// `data` must be either shorter than 32 bytes (left-padded with zeros to one
/// field element) or a concatenation of 32-byte big-endian field elements
/// (e.g. previous hashes), at most 64 of them. Returns the 32-byte big-endian
/// hash; aborts on invalid input.
#[inline]
pub fn mimc(data: impl AsRef<[u8]>) -> [u8; 32] {
    env::mimc(data.as_ref())
}

#[cfg(test)]
mod tests {
    #[test]
    fn mimc_native_mock() {
        crate::mock::reset();
        assert_eq!(super::mimc(b""), [0u8; 32]);
        assert_eq!(super::mimc(b"Hello World")[0], 37);
    }
}
