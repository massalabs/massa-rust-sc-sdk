//! Guest side of the Massa host <-> module memory protocol.
//!
//! Massa's runtime speaks the AssemblyScript object layout: a pointer points at
//! the payload and the payload byte length is stored as a little-endian `u32`
//! in the 4 bytes *before* it. Byte buffers (`StaticArray<u8>`) are raw bytes,
//! strings are UTF-16LE. The host allocates guest memory through the exported
//! `__new(size, class_id)` function and pins strings with `__pin`.
//!
//! Every object we hand to the host is therefore a `[len: u32][payload]` block
//! allocated with `Layout { size: len + 4, align: 4 }`, and every object the
//! host hands to us was created by `__new` with that same layout, so we can
//! release it once we copied the payload out.

use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::Layout;

const HEADER: usize = 4;

#[inline(always)]
fn layout_for(payload: usize) -> Layout {
    // SAFETY: align is a power of two and payload + 4 cannot overflow in wasm32.
    unsafe { Layout::from_size_align_unchecked(payload + HEADER, HEADER) }
}

#[inline(always)]
unsafe fn alloc_block(payload: usize) -> *mut u8 {
    let p = alloc::alloc::alloc(layout_for(payload));
    if p.is_null() {
        alloc::alloc::handle_alloc_error(layout_for(payload));
    }
    (p as *mut u32).write(payload as u32);
    p.add(HEADER)
}

/// Host-visible buffer owned by the guest; released when dropped.
pub(crate) struct HostBuf {
    ptr: *mut u8,
    alloc_size: usize,
}

impl HostBuf {
    /// Raw bytes (`StaticArray<u8>` on the host side).
    #[inline]
    pub fn bytes(data: &[u8]) -> Self {
        unsafe {
            let ptr = alloc_block(data.len());
            core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
            Self { ptr, alloc_size: data.len() }
        }
    }

    /// UTF-16LE string (`string` on the host side).
    pub fn string(s: &str) -> Self {
        // A UTF-16 encoding never needs more than 2 * utf8_len bytes.
        let max = s.len() * 2;
        unsafe {
            let ptr = alloc_block(max);
            let n = crate::codec::encode_utf16_raw(s, ptr as *mut u16);
            // Fix the header to the real byte length.
            (ptr.sub(HEADER) as *mut u32).write((n * 2) as u32);
            Self { ptr, alloc_size: max }
        }
    }

    #[inline(always)]
    pub fn ptr(&self) -> i32 {
        self.ptr as i32
    }
}

impl Drop for HostBuf {
    #[inline]
    fn drop(&mut self) {
        unsafe { alloc::alloc::dealloc(self.ptr.sub(HEADER), layout_for(self.alloc_size)) }
    }
}

#[inline(always)]
unsafe fn payload_len(ptr: i32) -> usize {
    ((ptr as usize - HEADER) as *const u32).read() as usize
}

#[inline(always)]
unsafe fn release(ptr: i32, len: usize) {
    alloc::alloc::dealloc((ptr as usize - HEADER) as *mut u8, layout_for(len));
}

/// Copy a host-provided byte buffer out and release the host block.
pub(crate) fn take_bytes(ptr: i32) -> Vec<u8> {
    if ptr <= 0 {
        return Vec::new();
    }
    unsafe {
        let len = payload_len(ptr);
        let v = core::slice::from_raw_parts(ptr as *const u8, len).to_vec();
        release(ptr, len);
        v
    }
}

/// Decode a host-provided UTF-16LE string and release the host block.
pub(crate) fn take_string(ptr: i32) -> String {
    if ptr <= 0 {
        return String::new();
    }
    unsafe {
        let len = payload_len(ptr);
        let units = core::slice::from_raw_parts(ptr as *const u16, len / 2);
        let s = crate::codec::utf16_to_string(units);
        release(ptr, len);
        s
    }
}

/// Allocate a buffer that is handed to the host as the function return value.
/// It is intentionally never freed: the instance ends right after.
pub(crate) fn leak_bytes(data: &[u8]) -> i32 {
    unsafe {
        let ptr = alloc_block(data.len());
        core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        ptr as i32
    }
}

/// Convert a 32-byte host hash buffer.
pub(crate) fn take_hash(ptr: i32) -> [u8; 32] {
    let v = take_bytes(ptr);
    let mut out = [0u8; 32];
    let n = v.len().min(32);
    out[..n].copy_from_slice(&v[..n]);
    out
}

// ---------------------------------------------------------------------------
// Exports expected by the host (AssemblyScript runtime interface subset).
// ---------------------------------------------------------------------------

/// Called by the host to allocate `size` bytes of guest memory.
#[no_mangle]
pub extern "C" fn __new(size: i32, _class_id: i32) -> i32 {
    unsafe { alloc_block(size as u32 as usize) as i32 }
}

/// GC pinning is meaningless here (no GC); the host still calls it for strings.
#[no_mangle]
pub extern "C" fn __pin(ptr: i32) -> i32 {
    ptr
}

#[no_mangle]
pub extern "C" fn __unpin(_ptr: i32) {}

#[no_mangle]
pub extern "C" fn __collect() {}
