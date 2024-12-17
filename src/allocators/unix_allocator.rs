use core::alloc::{GlobalAlloc, Layout};
use crate::allocators::generic_allocator::GenericAllocator;
use crate::blocklist::{Stats, Validity};

#[derive(Default)]
pub struct UnixAllocator {
    alloc: GenericAllocator<crate::allocators::heap_grower::EnhancedHeapGrower>,
}

impl UnixAllocator {
    #[inline(always)]
    pub const fn new() -> Self {
        UnixAllocator {
            alloc: GenericAllocator::new(),
        }
    }
    #[inline(always)]
    pub fn stats(&self) -> (Validity, Stats) {
        self.alloc.stats()
    }
}

unsafe impl GlobalAlloc for UnixAllocator {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc.get_raw().alloc(layout)
    }
    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc.get_raw().calloc(layout)
    }
    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.alloc.get_raw().realloc(ptr, layout, new_size)
    }
    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.alloc.get_raw().dealloc(ptr, layout)
    }
}
