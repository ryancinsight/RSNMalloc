use crate::allocators::raw_alloc::RawAlloc;
use crate::allocators::HeapGrower;
use crate::blocklist::{Stats, Validity};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};
use core::alloc::Layout;

#[repr(align(64))]
pub struct GenericAllocator<G: HeapGrower + Default> {
    init: AtomicU8,
    raw: MaybeUninit<RawAlloc<G>>,
}

pub struct AllocGuard<'a, G: HeapGrower + Default>(&'a mut RawAlloc<G>);

impl<'a, G: HeapGrower + Default> AllocGuard<'a, G> {
    #[inline(always)]
    pub fn stats(&self) -> (Validity, Stats) {
        self.0.stats()
    }

    #[inline(always)]
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        self.0.alloc(layout)
    }

    #[inline(always)]
    pub unsafe fn calloc(&mut self, layout: Layout) -> *mut u8 {
        self.0.calloc(layout)
    }

    #[inline(always)]
    pub unsafe fn realloc(&mut self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.0.realloc(ptr, layout, new_size)
    }

    #[inline(always)]
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout)
    }
}

impl<G: HeapGrower + Default> GenericAllocator<G> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            init: AtomicU8::new(0),
            raw: MaybeUninit::uninit(),
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> (Validity, Stats) {
        unsafe { self.get_raw().stats() }
    }

    #[inline(always)]
    pub unsafe fn get_raw(&self) -> AllocGuard<G> {
        // Fast path: Check initialization state.
        let state = self.init.load(Ordering::Relaxed);
        if state == 2 {
            return AllocGuard(&mut *(self.raw.as_ptr() as *mut RawAlloc<G>));
        }
        self.ensure_initialized()
    }

    #[inline(always)]
    unsafe fn ensure_initialized(&self) -> AllocGuard<G> {
        // Attempt initialization.
        if self.init.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            let raw_ptr = self.raw.as_ptr() as *mut RawAlloc<G>;
            raw_ptr.write(RawAlloc::default());
            self.init.store(2, Ordering::Release);
            return AllocGuard(&mut *raw_ptr);
        }

        // Wait for another thread to finish initialization.
        let mut backoff = 4;
        loop {
            if self.init.load(Ordering::Acquire) == 2 {
                // Initialization complete.
                return AllocGuard(&mut *(self.raw.as_ptr() as *mut RawAlloc<G>));
            }

            // Cooperative waiting with dynamic step size based on backoff
            for _ in (0..backoff).step_by(backoff.min(4)) {
                core::hint::spin_loop();
            }
            backoff = (backoff * 2).min(64); // Capped exponential backoff

        }
    }
}

impl<G: HeapGrower + Default> Default for GenericAllocator<G> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<G: HeapGrower + Default> Send for GenericAllocator<G> {}
unsafe impl<G: HeapGrower + Default> Sync for GenericAllocator<G> {}