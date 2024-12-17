use core::sync::atomic::{AtomicUsize, Ordering};
use crate::allocators::atomic_array::AtomicArray;
use crate::allocators::heap_grower::HeapGrower;

pub struct ToyHeap {
    pub page_size: usize,
    pub size: AtomicUsize,
    pub heap: AtomicArray,
}

impl Default for ToyHeap {
    fn default() -> Self {
        ToyHeap {
            page_size: 64,
            size: AtomicUsize::new(0),
            heap: AtomicArray::new(256 * 1024),
        }
    }
}

pub struct ToyHeapOverflowError();

impl HeapGrower for ToyHeap {
    type Err = ToyHeapOverflowError;

    unsafe fn grow_heap(&mut self, size: usize) -> Result<(*mut u8, usize), ToyHeapOverflowError> {
        let allocating = round_up(size, self.page_size);
        
        // Atomically fetch and update the size
        let current_size = self.size.fetch_add(allocating, Ordering::SeqCst);
        
        if current_size + allocating > self.heap.len() {
            // Roll back if allocation exceeds heap size
            self.size.fetch_sub(allocating, Ordering::SeqCst);
            return Err(ToyHeapOverflowError());
        }

        // Return a pointer to the allocated region
        let ptr = self.heap.as_ptr().add(current_size) as *mut u8;
        Ok((ptr, allocating))
    }
}

fn round_up(value: usize, increment: usize) -> usize {
    (value + increment - 1) / increment * increment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allocators::raw_alloc::RawAlloc;
    use core::alloc::Layout;
    use core::ptr::null_mut;
    use test_log::test;

    #[test]
    fn test_basic() {
        let toy_heap = ToyHeap::default();
        let mut allocator = RawAlloc::new(toy_heap);
        
        const BLOCKS: usize = 3;
        let layouts: [Layout; BLOCKS] = [
            Layout::from_size_align(64, 16).unwrap(),
            Layout::from_size_align(64, 16).unwrap(),
            Layout::from_size_align(224, 16).unwrap(),
        ];

        let pointers: [*mut u8; BLOCKS] = unsafe {
            let mut pointers = [null_mut(); BLOCKS];
            for (i, &l) in layouts.iter().enumerate() {
                pointers[i] = allocator.alloc(l);
                let (validity, _stats) = allocator.stats();
                assert!(validity.is_valid());
            }
            pointers
        };

        for i in 0..BLOCKS - 1 {
            let l = layouts[i];
            let expected = unsafe { pointers[i].add(l.size()) };
            let found = pointers[i + 1];
            assert_eq!(expected, found);
        }

        let toy_heap = &allocator.grower;
        let page_size = toy_heap.page_size;
        let total_allocated: usize = layouts.iter().map(|l| l.size()).sum();
        let page_space = round_up(total_allocated, page_size);
        assert_eq!(toy_heap.size.load(Ordering::Relaxed), page_space);
    }
}
