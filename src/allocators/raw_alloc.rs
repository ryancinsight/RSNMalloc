use core::alloc::Layout;
use core::ptr::{null_mut, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::blocklist::{BlockList, Stats, Validity};
use crate::allocators::heap_grower::HeapGrower;

#[repr(align(64))]
pub struct RawAlloc<G: HeapGrower> {
    pub grower: G,
    pub blocks: BlockList,
    allocation_counter: AtomicUsize,
    deallocation_counter: AtomicUsize,
}

impl<G: HeapGrower> Drop for RawAlloc<G> {
    #[inline(always)]
    fn drop(&mut self) {
        let blocks = core::mem::take(&mut self.blocks);
        core::mem::forget(blocks);
    }
}

impl<G: HeapGrower + Default> Default for RawAlloc<G> {
    #[inline(always)]
    fn default() -> Self {
        RawAlloc {
            grower: G::default(),
            blocks: BlockList::default(),
            allocation_counter: AtomicUsize::new(0),
            deallocation_counter: AtomicUsize::new(0),
        }
    }
}

impl<G: HeapGrower> RawAlloc<G> {
    #[inline(always)]
    pub fn new(grower: G) -> Self {
        RawAlloc {
            grower,
            blocks: BlockList::default(),
            allocation_counter: AtomicUsize::new(0),
            deallocation_counter: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> (Validity, Stats) {
        self.blocks.stats()
    }

    #[inline(always)]
    pub fn block_size(layout: Layout) -> usize {
        let aligned_layout = layout
            .align_to(16)
            .expect("Alignment failed")
            .pad_to_align();
        aligned_layout.size()
    }
    #[inline]
    pub fn allocation_count(&self) -> usize {
        let count = self.allocation_counter.load(Ordering::Relaxed);
        count
    } 
    #[inline]
    pub fn deallocation_count(&self) -> usize {
        let count = self.deallocation_counter.load(Ordering::Relaxed);
        count
    }

    #[inline(always)]
    unsafe fn try_expand_allocation(
        &mut self,
        ptr: *mut u8,
        old_size: usize,
        new_block_size: usize
    ) -> Option<*mut u8> {
        if let Some(block) = self.blocks
            .iter()
            .find(|block| block.as_range().start as *mut u8 == ptr.add(old_size))
        {
            let block_size = block.size();
            if old_size.wrapping_add(block_size) >= new_block_size {
                self.blocks.pop_size(block_size);
                return Some(ptr);
            }
        }
        None
    }

    #[inline(always)]
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        self.allocation_counter.fetch_add(1, Ordering::Relaxed);
        let needed_size = Self::block_size(layout);

        if let Some(range) = self.blocks.pop_size(needed_size) {
            return range.start.as_ptr();
        }

        match self.grower.grow_heap(needed_size) {
            Err(_) => {
                self.allocation_counter.fetch_sub(1, Ordering::Relaxed);
                null_mut()
            },
            Ok((ptr, size)) => {
                if size >= needed_size.wrapping_add(BlockList::header_size()) {
                    let free_ptr = NonNull::new_unchecked(ptr.add(needed_size));
                    self.blocks.add_block(free_ptr, size.wrapping_sub(needed_size));
                }
                ptr
            }
        }
    }

    #[inline(always)]
    pub unsafe fn calloc(&mut self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }

    #[inline(always)]
    pub unsafe fn realloc(&mut self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
        }

        let old_size = Self::block_size(layout);
        let new_block_size = Self::round_up(new_size, 16);

        if new_block_size <= old_size {
            if new_block_size.wrapping_add(BlockList::header_size()) <= old_size {
                let free_ptr = NonNull::new_unchecked(ptr.add(new_block_size));
                self.blocks.add_block(free_ptr, old_size.wrapping_sub(new_block_size));
            }
            return ptr;
        }

        if let Some(expanded_ptr) = self.try_expand_allocation(ptr, old_size, new_block_size) {
            return expanded_ptr;
        }

        self.allocation_counter.fetch_add(1, Ordering::Relaxed);
        let new_ptr = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
        if !new_ptr.is_null() {
            core::ptr::copy_nonoverlapping(
                ptr,
                new_ptr,
                core::cmp::min(old_size, new_block_size)
            );
            self.dealloc(ptr, layout);
        } else {
            self.allocation_counter.fetch_sub(1, Ordering::Relaxed);
        }
        new_ptr
    }

    #[inline(always)]
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            debug_assert!(false, "Attempted to deallocate a null pointer");
            return;
        }

        self.deallocation_counter.fetch_add(1, Ordering::Relaxed);
        let size = Self::block_size(layout);

        debug_assert!(
            ptr.align_offset(layout.align()) == 0,
            "Deallocation with improper alignment"
        );

        debug_assert!({
            let mut is_double_free = false;
            for block in self.blocks.iter() {
                let block_range = block.as_range();
                if block_range.start as *mut u8 <= ptr && ptr < block_range.end as *mut u8 {
                    is_double_free = true;
                    break;
                }
            }
            !is_double_free
        }, "Double free detected");

        #[cfg(debug_assertions)]
        core::ptr::write_bytes(ptr, 0, size);

        self.blocks.add_block(NonNull::new_unchecked(ptr), size);
    }

    #[inline(always)]
    const fn round_up(value: usize, increment: usize) -> usize {
        value.wrapping_add(increment.wrapping_sub(1))
            .wrapping_div(increment)
            .wrapping_mul(increment)
    }
}
