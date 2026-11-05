//! Events: strings recorded with the execution output and readable off-chain.
//! Keep them short (the node caps event size, 1 KiB on current networks).

use alloc::string::String;
use alloc::vec::Vec;

use crate::env;

/// Emit `event`.
#[inline]
pub fn generate_event(event: &str) {
    env::generate_event(event)
}

/// Emit binary data as a base64 event (standard alphabet, padded), like the
/// AssemblyScript `generateRawEvent`.
pub fn generate_raw_event(data: &[u8]) {
    env::generate_event(&base64_encode(data))
}

/// Format an event as `key:arg1,arg2,...` (AssemblyScript `createEvent`).
pub fn create_event(key: &str, args: &[&str]) -> String {
    let mut s = String::with_capacity(key.len() + 1 + args.iter().map(|a| a.len() + 1).sum::<usize>());
    s.push_str(key);
    s.push(':');
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(a);
    }
    s
}

/// Standard base64 encoding with padding.
pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63]);
        out.push(TABLE[(n >> 12) as usize & 63]);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] } else { b'=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] } else { b'=' });
    }
    // SAFETY: only ASCII bytes were pushed.
    unsafe { String::from_utf8_unchecked(out) }
}

/// Decode standard base64 (padding optional). `None` on invalid input.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    }
    let bytes = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zg").unwrap(), b"f");
        assert_eq!(base64_decode("Z"), None);
        assert_eq!(base64_decode("Zg!="), None);
    }

    #[test]
    fn event_format() {
        assert_eq!(create_event("TRANSFER", &["a", "b"]), "TRANSFER:a,b");
        assert_eq!(create_event("X", &[]), "X:");
        crate::mock::reset();
        generate_event("hello");
        generate_raw_event(b"foo");
        assert_eq!(crate::mock::events(), vec!["hello", "Zm9v"]);
    }
}
