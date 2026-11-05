//! A tiny size-class allocator tuned for smart-contract executions.
//!
//! Massa executions are single-threaded, short-lived and capped at 4 MiB of
//! linear memory. General-purpose allocators (dlmalloc, talc, ...) cost
//! 5-10 KiB of bytecode and tens of thousands of gas per execution. This
//! allocator costs a few hundred bytes and a handful of instructions per
//! allocation, while still recycling memory:
//!
//! * blocks are rounded up to power-of-two size classes (16 B .. 512 KiB) and
//!   freed blocks are kept in per-class free lists (O(1) reuse),
//! * freeing or growing the most recent allocation simply moves the bump
//!   pointer (the common LIFO / `Vec` growth pattern costs nothing),
//! * blocks larger than 512 KiB are bump-allocated with an exact size.
//!
//! Memory is obtained from `memory.grow` on demand. It is never returned to
//! the engine; the instance is discarded at the end of the execution anyway.

#[cfg(target_arch = "wasm32")]
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::marker::PhantomData;

const MIN_SHIFT: u32 = 4; // smallest class: 16 bytes
const NUM_CLASSES: usize = 16; // 16 B .. 512 KiB
const LARGE: usize = 1 << (MIN_SHIFT + NUM_CLASSES as u32 - 1);
const MAX_ALIGN: usize = 1 << MIN_SHIFT;
#[cfg(any(target_arch = "wasm32", test))]
const PAGE: usize = 65536;

/// Where the heap lives. Implemented for the wasm linear memory and, in
/// tests, for a static arena.
pub trait Backend {
    /// First usable address (will be rounded up to 16 bytes).
    fn base() -> usize;
    /// Current end of the usable address space.
    fn limit() -> usize;
    /// Extend the address space so that it reaches at least `wanted`.
    /// Returns the new limit, or `None` on failure.
    fn grow(wanted: usize) -> Option<usize>;
}

pub struct Heap<B: Backend> {
    top: usize,
    limit: usize,
    free: [usize; NUM_CLASSES],
    _b: PhantomData<B>,
}

#[inline(always)]
fn class(size: usize) -> (usize, usize) {
    let size = if size < MAX_ALIGN { MAX_ALIGN } else { size };
    if size > LARGE {
        return (usize::MAX, (size + (MAX_ALIGN - 1)) & !(MAX_ALIGN - 1));
    }
    let rounded = size.next_power_of_two();
    ((rounded.trailing_zeros() - MIN_SHIFT) as usize, rounded)
}

impl<B: Backend> Heap<B> {
    pub const fn new() -> Self {
        Self { top: 0, limit: 0, free: [0; NUM_CLASSES], _b: PhantomData }
    }

    #[inline(always)]
    fn init(&mut self) {
        if self.top == 0 {
            self.top = (B::base() + (MAX_ALIGN - 1)) & !(MAX_ALIGN - 1);
            self.limit = B::limit();
        }
    }

    /// Make sure addresses below `end` are usable.
    #[inline(always)]
    fn reserve(&mut self, end: usize) -> bool {
        if end <= self.limit {
            return true;
        }
        match B::grow(end) {
            Some(limit) => {
                self.limit = limit;
                true
            }
            None => false,
        }
    }

    #[inline(always)]
    unsafe fn bump(&mut self, size: usize, align: usize) -> *mut u8 {
        let start = (self.top + align - 1) & !(align - 1);
        let end = start + size;
        if !self.reserve(end) {
            return core::ptr::null_mut();
        }
        self.top = end;
        start as *mut u8
    }

    /// # Safety
    /// `layout` must have a non-zero size; the returned block is uninitialized.
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        self.init();
        let (idx, rounded) = class(layout.size());
        let align = layout.align();
        if align <= MAX_ALIGN {
            if idx != usize::MAX {
                let head = self.free[idx];
                if head != 0 {
                    self.free[idx] = *(head as *const usize);
                    return head as *mut u8;
                }
            }
            self.bump(rounded, MAX_ALIGN)
        } else {
            self.bump(rounded, align)
        }
    }

    /// # Safety
    /// `ptr` must come from [`Self::alloc`]/[`Self::realloc`] with the same `layout`.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let p = ptr as usize;
        let (idx, rounded) = class(layout.size());
        if p + rounded == self.top {
            self.top = p;
            return;
        }
        if idx != usize::MAX && layout.align() <= MAX_ALIGN {
            *(p as *mut usize) = self.free[idx];
            self.free[idx] = p;
        }
    }

    /// # Safety
    /// Same contract as [`core::alloc::GlobalAlloc::realloc`].
    pub unsafe fn realloc(&mut self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = ptr as usize;
        let (_, old_rounded) = class(layout.size());
        let (_, new_rounded) = class(new_size);
        if new_rounded == old_rounded {
            return ptr;
        }
        if p + old_rounded == self.top && layout.align() <= MAX_ALIGN {
            // Most recent allocation: resize in place.
            let end = p + new_rounded;
            if !self.reserve(end) {
                return core::ptr::null_mut();
            }
            self.top = end;
            return ptr;
        }
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            let n = if layout.size() < new_size { layout.size() } else { new_size };
            core::ptr::copy_nonoverlapping(ptr, new_ptr, n);
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    pub struct WasmMemory;

    impl Backend for WasmMemory {
        #[inline(always)]
        fn base() -> usize {
            extern "C" {
                static __heap_base: u8;
            }
            core::ptr::addr_of!(__heap_base) as usize
        }
        #[inline(always)]
        fn limit() -> usize {
            core::arch::wasm32::memory_size(0) * PAGE
        }
        #[inline(always)]
        fn grow(wanted: usize) -> Option<usize> {
            let have = Self::limit();
            let pages = (wanted - have).div_ceil(PAGE);
            if core::arch::wasm32::memory_grow(0, pages) == usize::MAX {
                None
            } else {
                Some(have + pages * PAGE)
            }
        }
    }

    /// The global allocator installed by the `allocator` feature.
    pub struct ContractAllocator;

    static mut HEAP: Heap<WasmMemory> = Heap::new();

    unsafe impl GlobalAlloc for ContractAllocator {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            (*core::ptr::addr_of_mut!(HEAP)).alloc(layout)
        }
        #[inline]
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            (*core::ptr::addr_of_mut!(HEAP)).dealloc(ptr, layout)
        }
        #[inline]
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            (*core::ptr::addr_of_mut!(HEAP)).realloc(ptr, layout, new_size)
        }
    }

    #[cfg(feature = "allocator")]
    #[global_allocator]
    static GLOBAL: ContractAllocator = ContractAllocator;
}

#[cfg(target_arch = "wasm32")]
pub use wasm::ContractAllocator;

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    const ARENA_SIZE: usize = 8 << 20;
    #[repr(align(16))]
    #[allow(dead_code)]
    struct Arena([u8; ARENA_SIZE]);
    static mut ARENA: Arena = Arena([0; ARENA_SIZE]);
    static mut ARENA_LIMIT: usize = 0;

    struct TestBackend;
    impl Backend for TestBackend {
        fn base() -> usize {
            core::ptr::addr_of!(ARENA) as usize
        }
        fn limit() -> usize {
            unsafe { ARENA_LIMIT }
        }
        fn grow(wanted: usize) -> Option<usize> {
            let end = Self::base() + ARENA_SIZE;
            if wanted > end {
                return None;
            }
            let new = (Self::base() + ((wanted - Self::base()).div_ceil(PAGE)) * PAGE).min(end);
            unsafe { ARENA_LIMIT = new };
            Some(new)
        }
    }

    // The arena is shared by every test of this module: run them one at a time.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fresh() -> (std::sync::MutexGuard<'static, ()>, Heap<TestBackend>) {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { ARENA_LIMIT = TestBackend::base() + PAGE };
        (guard, Heap::new())
    }

    #[test]
    fn classes() {
        assert_eq!(class(0), (0, 16));
        assert_eq!(class(1), (0, 16));
        assert_eq!(class(16), (0, 16));
        assert_eq!(class(17), (1, 32));
        assert_eq!(class(1000), (6, 1024));
        assert_eq!(class(LARGE), (NUM_CLASSES - 1, LARGE));
        assert_eq!(class(LARGE + 1), (usize::MAX, LARGE + 16));
    }

    #[test]
    fn lifo_rewinds_and_free_lists_recycle() {
        let (_guard, mut h) = fresh();
        unsafe {
            let l = Layout::from_size_align(100, 8).unwrap();
            let a = h.alloc(l);
            let b = h.alloc(l);
            assert_eq!(b as usize, a as usize + 128);
            h.dealloc(b, l); // top -> rewind
            let b2 = h.alloc(l);
            assert_eq!(b2, b);
            h.dealloc(a, l); // not top -> free list
            let c = h.alloc(l); // recycled
            assert_eq!(c, a);
            // realloc of the top block grows in place
            let d = h.realloc(b2, l, 300);
            assert_eq!(d, b2);
            assert_eq!(h.top, b2 as usize + 512);
            // realloc inside the same class is a no-op
            assert_eq!(h.realloc(c, l, 120), c);
        }
    }

    #[test]
    fn large_blocks_and_alignment() {
        let (_guard, mut h) = fresh();
        unsafe {
            let big = Layout::from_size_align(LARGE + 5, 1).unwrap();
            let p = h.alloc(big);
            assert!(!p.is_null());
            assert_eq!(p as usize % 16, 0);
            h.dealloc(p, big);
            assert_eq!(h.top, p as usize);
            let aligned = Layout::from_size_align(8, 64).unwrap();
            let q = h.alloc(aligned);
            assert_eq!(q as usize % 64, 0);
            // out of memory is reported, not UB
            let huge = Layout::from_size_align(ARENA_SIZE * 2, 1).unwrap();
            assert!(h.alloc(huge).is_null());
        }
    }

    #[test]
    fn randomized_stress() {
        // Deterministic xorshift to avoid pulling a rand dependency.
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let (_guard, mut h) = fresh();
        let mut live: Vec<(*mut u8, Layout, u8)> = Vec::new();
        for i in 0..20_000u32 {
            let r = rnd();
            if live.is_empty() || r % 3 != 0 {
                let size = (rnd() % 700) as usize + 1;
                let align = 1usize << (rnd() % 5);
                let l = Layout::from_size_align(size, align).unwrap();
                let p = unsafe { h.alloc(l) };
                assert!(!p.is_null());
                assert_eq!(p as usize % align, 0);
                let tag = (i % 251) as u8;
                unsafe { core::ptr::write_bytes(p, tag, size) };
                live.push((p, l, tag));
            } else if r % 7 == 0 {
                let idx = (rnd() as usize) % live.len();
                let (p, l, tag) = live[idx];
                let new_size = (rnd() % 1500) as usize + 1;
                let np = unsafe { h.realloc(p, l, new_size) };
                assert!(!np.is_null());
                let keep = l.size().min(new_size);
                for k in 0..keep {
                    assert_eq!(unsafe { *np.add(k) }, tag, "realloc lost data");
                }
                let nl = Layout::from_size_align(new_size, l.align()).unwrap();
                unsafe { core::ptr::write_bytes(np, tag, new_size) };
                live[idx] = (np, nl, tag);
            } else {
                let idx = (rnd() as usize) % live.len();
                let (p, l, tag) = live.swap_remove(idx);
                for k in 0..l.size() {
                    assert_eq!(unsafe { *p.add(k) }, tag, "block corrupted");
                }
                unsafe { h.dealloc(p, l) };
            }
            // No two live blocks may overlap.
            if i % 997 == 0 {
                let mut spans: Vec<(usize, usize)> =
                    live.iter().map(|(p, l, _)| (*p as usize, *p as usize + l.size())).collect();
                spans.sort();
                for w in spans.windows(2) {
                    assert!(w[0].1 <= w[1].0, "overlap");
                }
            }
        }
    }
}
