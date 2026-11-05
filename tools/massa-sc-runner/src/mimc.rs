//! MiMC over the BN254 scalar field (C-Layer `assembly_script_hash_mimc`).
#![allow(dead_code)]
use ethnum::U256;
use sha3::Digest;

// BN254 scalar field modulus r.
const R: U256 = U256::from_words(0x30644e72e131a029b85045b68181585d, 0x2833e84879b9709143e1f593f0000001);
const ROUNDS: usize = 110;

fn add(a: U256, b: U256) -> U256 {
    let s = a + b; // < 2 * R < 2^255: no overflow
    if s >= R {
        s - R
    } else {
        s
    }
}

fn mul(a: U256, b: U256) -> U256 {
    let mut acc = U256::ZERO;
    for i in (0..256).rev() {
        acc = add(acc, acc);
        if (b >> i) & U256::ONE == U256::ONE {
            acc = add(acc, a);
        }
    }
    acc
}

fn pow5(x: U256) -> U256 {
    let x2 = mul(x, x);
    let x4 = mul(x2, x2);
    mul(x4, x)
}

fn from_be_mod(bytes: &[u8; 32]) -> U256 {
    U256::from_be_bytes(*bytes) % R
}

pub fn hash(bytes: &[u8]) -> Result<[u8; 32], String> {
    if bytes.len() % 32 != 0 && bytes.len() > 32 {
        return Err("Input length must less than 32 or a multiple of 32".into());
    }
    if bytes.len() > 64 * 32 {
        return Err("Input length must less than 2048".into());
    }
    // round constants
    let mut hasher = sha3::Keccak256::new();
    hasher.update(b"seed");
    let mut rnd = hasher.finalize_reset();
    hasher.update(rnd);
    let mut constants = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        rnd = hasher.finalize_reset();
        constants.push(from_be_mod(&rnd.into()));
        hasher.update(rnd);
    }
    // data elements
    let mut elements = Vec::new();
    if !bytes.is_empty() {
        if bytes.len() < 32 {
            let mut padded = [0u8; 32];
            padded[32 - bytes.len()..].copy_from_slice(bytes);
            elements.push(from_be_mod(&padded));
        } else {
            for chunk in bytes.chunks(32) {
                let arr: [u8; 32] = chunk.try_into().unwrap();
                let v = U256::from_be_bytes(arr);
                if v >= R {
                    return Err("Cannot convert into field element".into());
                }
                elements.push(v);
            }
        }
    }
    let mut h = U256::ZERO;
    for e in elements {
        let mut m = e;
        for c in &constants {
            m = pow5(add(add(m, h), *c));
        }
        m = add(m, h);
        h = add(add(m, h), e);
    }
    Ok(h.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use super::hash;

    #[test]
    fn matches_clayer_node_vectors() {
        assert_eq!(hash(b"").unwrap(), [0u8; 32]);
        let h1 = hash(b"Hello World").unwrap();
        assert_eq!(
            h1,
            [
                37, 9, 120, 104, 178, 123, 147, 205, 1, 53, 154, 155, 95, 126, 42, 115, 62, 136, 182, 12,
                227, 45, 22, 153, 180, 233, 123, 101, 206, 135, 162, 184
            ]
        );
        assert_eq!(
            hash(&h1).unwrap(),
            [
                32, 230, 249, 72, 11, 8, 251, 253, 161, 246, 128, 243, 21, 63, 102, 187, 2, 212, 194, 126,
                100, 195, 9, 100, 15, 135, 217, 110, 106, 38, 120, 245
            ]
        );
        assert_eq!(
            hash(&[h1, h1].concat()).unwrap(),
            [
                31, 13, 212, 12, 150, 157, 144, 161, 249, 21, 94, 81, 53, 123, 172, 234, 151, 64, 249, 134,
                149, 179, 241, 241, 49, 37, 128, 22, 128, 233, 208, 51
            ]
        );
    }
}
