//! Hand-rolled text codecs for the host boundary. Addresses, function names
//! and most events are ASCII, and every executed wasm instruction costs gas,
//! so these avoid the generic iterator machinery of `core`.

use alloc::string::String;
use alloc::vec::Vec;

/// Decode one UTF-8 scalar starting at `bytes[i]` (input must be valid UTF-8
/// and `bytes[i] >= 0x80`).
#[inline(always)]
fn decode_utf8_at(bytes: &[u8], i: usize) -> (u32, usize) {
    let b0 = bytes[i] as u32;
    if b0 < 0xE0 {
        ((b0 & 0x1F) << 6 | (bytes[i + 1] as u32 & 0x3F), 2)
    } else if b0 < 0xF0 {
        ((b0 & 0x0F) << 12 | (bytes[i + 1] as u32 & 0x3F) << 6 | (bytes[i + 2] as u32 & 0x3F), 3)
    } else {
        (
            (b0 & 0x07) << 18
                | (bytes[i + 1] as u32 & 0x3F) << 12
                | (bytes[i + 2] as u32 & 0x3F) << 6
                | (bytes[i + 3] as u32 & 0x3F),
            4,
        )
    }
}

/// Encode `s` as UTF-16 code units into `out`, which must have room for at
/// least `s.len()` units. Returns the number of units written.
///
/// # Safety
/// `out` must be valid for `s.len()` writes of `u16` (unaligned writes are used).
pub unsafe fn encode_utf16_raw(s: &str, out: *mut u16) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut n = 0usize;
    while i < bytes.len() {
        let b = *bytes.get_unchecked(i);
        if b < 0x80 {
            out.add(n).write_unaligned(b as u16);
            i += 1;
            n += 1;
            continue;
        }
        let (cp, len) = decode_utf8_at(bytes, i);
        i += len;
        if cp >= 0x1_0000 {
            let c = cp - 0x1_0000;
            out.add(n).write_unaligned(0xD800 | (c >> 10) as u16);
            out.add(n + 1).write_unaligned(0xDC00 | (c & 0x3FF) as u16);
            n += 2;
        } else {
            out.add(n).write_unaligned(cp as u16);
            n += 1;
        }
    }
    n
}

/// UTF-16 -> `String` with an ASCII fast path; lone surrogates become U+FFFD.
/// Trailing NUL units are dropped: the host sizes string buffers from the
/// UTF-8 length, so non-ASCII strings carry zero padding.
pub fn utf16_to_string(units: &[u16]) -> String {
    let mut end = units.len();
    while end > 0 && units[end - 1] == 0 {
        end -= 1;
    }
    let units = &units[..end];
    let mut out: Vec<u8> = Vec::with_capacity(units.len() * 3);
    // SAFETY: at most 3 bytes are written per unit (4 per surrogate pair, i.e.
    // 2 units), within the reserved capacity; only well-formed UTF-8 is produced.
    unsafe {
        let p = out.as_mut_ptr();
        let mut n = 0usize;
        let mut i = 0usize;
        while i < units.len() {
            let u = *units.get_unchecked(i) as u32;
            i += 1;
            if u < 0x80 {
                *p.add(n) = u as u8;
                n += 1;
                continue;
            }
            let cp = if (0xD800..0xDC00).contains(&u) {
                match units.get(i) {
                    Some(&lo) if (0xDC00..0xE000).contains(&lo) => {
                        i += 1;
                        0x1_0000 + ((u - 0xD800) << 10) + (lo as u32 - 0xDC00)
                    }
                    _ => 0xFFFD,
                }
            } else if (0xDC00..0xE000).contains(&u) {
                0xFFFD
            } else {
                u
            };
            if cp < 0x800 {
                *p.add(n) = (0xC0 | (cp >> 6)) as u8;
                *p.add(n + 1) = (0x80 | (cp & 0x3F)) as u8;
                n += 2;
            } else if cp < 0x1_0000 {
                *p.add(n) = (0xE0 | (cp >> 12)) as u8;
                *p.add(n + 1) = (0x80 | ((cp >> 6) & 0x3F)) as u8;
                *p.add(n + 2) = (0x80 | (cp & 0x3F)) as u8;
                n += 3;
            } else {
                *p.add(n) = (0xF0 | (cp >> 18)) as u8;
                *p.add(n + 1) = (0x80 | ((cp >> 12) & 0x3F)) as u8;
                *p.add(n + 2) = (0x80 | ((cp >> 6) & 0x3F)) as u8;
                *p.add(n + 3) = (0x80 | (cp & 0x3F)) as u8;
                n += 4;
            }
        }
        out.set_len(n);
        String::from_utf8_unchecked(out)
    }
}

/// Extract the string elements of a JSON array of strings (`["a","b"]`) as
/// produced by the host for call stacks and owned addresses (no escapes).
pub fn json_string_array(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            // SAFETY: slicing between two ASCII quotes of a valid str keeps it valid.
            out.push(String::from(unsafe { s.get_unchecked(start..j) }));
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// `n`-th string element counted from the end of a JSON string array
/// (`0` = last). Allocates only the returned element.
pub fn json_string_array_from_end(s: &str, n: usize) -> Option<String> {
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    let mut remaining = n;
    loop {
        // closing quote
        while end > 0 && bytes[end - 1] != b'"' {
            end -= 1;
        }
        if end == 0 {
            return None;
        }
        let close = end - 1;
        let mut start = close;
        while start > 0 && bytes[start - 1] != b'"' {
            start -= 1;
        }
        if start == 0 {
            return None;
        }
        if remaining == 0 {
            // SAFETY: slicing between two ASCII quotes of a valid str keeps it valid.
            return Some(String::from(unsafe { s.get_unchecked(start..close) }));
        }
        remaining -= 1;
        end = start - 1;
    }
}

/// Decode the host's serialized key list:
/// `count: u32 LE` then `count` times `len: u8, key: [u8; len]`.
/// An empty buffer means no key.
pub fn decode_keys(buf: &[u8]) -> Vec<Vec<u8>> {
    if buf.len() < 4 {
        return Vec::new();
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let mut keys = Vec::with_capacity(count);
    let mut cursor = 4;
    for _ in 0..count {
        let Some(&len) = buf.get(cursor) else { break };
        let start = cursor + 1;
        let end = start + len as usize;
        if end > buf.len() {
            break;
        }
        keys.push(buf[start..end].to_vec());
        cursor = end;
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(s: &str) -> Vec<u16> {
        let mut v = alloc::vec![0u16; s.len()];
        let n = unsafe { encode_utf16_raw(s, v.as_mut_ptr()) };
        v.truncate(n);
        v
    }

    #[test]
    fn utf16_roundtrips() {
        for s in ["", "hello", "héllo wörld", "日本語", "emoji 😀 ok", "\u{10FFFF}"] {
            let units = encode(s);
            assert_eq!(units, s.encode_utf16().collect::<Vec<_>>(), "{s}");
            assert_eq!(utf16_to_string(&units), s);
        }
    }

    #[test]
    fn utf16_padding_and_invalid() {
        assert_eq!(utf16_to_string(&[b'a' as u16, 0, 0]), "a");
        assert_eq!(utf16_to_string(&[0xD800, b'x' as u16]), "\u{FFFD}x");
        assert_eq!(utf16_to_string(&[0xDC00]), "\u{FFFD}");
    }

    #[test]
    fn json_arrays() {
        assert_eq!(json_string_array("[]"), Vec::<String>::new());
        assert_eq!(json_string_array("[\"AU1\",\"AS2\"]"), vec!["AU1", "AS2"]);
        assert_eq!(json_string_array("[ \"AU1\" , \"AS2\" ]"), vec!["AU1", "AS2"]);
        let s = "[\"AU1\",\"AS2\",\"AS3\"]";
        assert_eq!(json_string_array_from_end(s, 0).as_deref(), Some("AS3"));
        assert_eq!(json_string_array_from_end(s, 1).as_deref(), Some("AS2"));
        assert_eq!(json_string_array_from_end(s, 2).as_deref(), Some("AU1"));
        assert_eq!(json_string_array_from_end(s, 3), None);
        assert_eq!(json_string_array_from_end("[]", 0), None);
    }

    #[test]
    fn keys_decoding() {
        assert!(decode_keys(&[]).is_empty());
        assert_eq!(decode_keys(&[2, 0, 0, 0, 3, 1, 2, 3, 1, 255]), vec![vec![1, 2, 3], vec![255]]);
        assert_eq!(decode_keys(&[2, 0, 0, 0, 3, 1, 2, 3, 0]), vec![vec![1, 2, 3], vec![]]);
    }
}
