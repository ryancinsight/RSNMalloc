use core::ops::Range;
use core::ptr::NonNull;
use super::free_header::{FreeHeader, header_size};
use core::sync::atomic::Ordering;
use crate::relation::Relation;
#[derive(Debug)]
#[repr(transparent)]
pub struct FreeBlock {
    pub header: NonNull<FreeHeader>,
}

impl Drop for FreeBlock {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            {
                let slice = core::slice::from_raw_parts_mut(
                    self.header.as_ptr() as *mut u8,
                    self.size()
                );
                slice.fill(0);
            }
            
            self.header_mut().next=None;
            self.header_mut().size.store(0, core::sync::atomic::Ordering::Release);
        }
    }
}

impl FreeBlock {
    // Use const evaluation where possible
    const MIN_SPLIT_SIZE: usize = header_size();
    const BATCH_SIZE: usize = 8;
    #[inline(always)]
    pub fn can_split(&self, size: usize) -> bool {
        self.size() >= size.wrapping_add(super::free_header::header_size())
    }
    #[inline]
    pub fn relation(&self, other: &Self) -> Relation {
        let self_size = self.size();
        let other_size = other.size();

        let self_start = self.as_range().start as usize;
        let self_end = self_start.wrapping_add(self_size);
        let other_start = other.as_range().start as usize;
        let other_end = other_start.wrapping_add(other_size);

        match () {
            _ if self_end < other_start => Relation::Before,
            _ if self_end == other_start => Relation::AdjacentBefore,
            _ if self_start < other_end => Relation::Overlapping,
            _ if self_start == other_end => Relation::AdjacentAfter,
            _ => Relation::After,
        }
    }
    #[must_use]
    #[inline]
    pub unsafe fn from_raw(ptr: NonNull<u8>, next: Option<FreeBlock>, size: usize) -> FreeBlock {
        debug_assert!(
            size >= header_size(),
            "Can't recapture a block smaller than HEADER_SIZE"
        );
        let header = FreeHeader::from_raw(ptr, next, size);
        FreeBlock { header }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let size = self.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
            core::slice::from_raw_parts(self.header.as_ptr() as *const u8, size)
        }
    }

    #[inline(always)]
    pub fn as_range(&self) -> Range<*const u8> {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        unsafe {
            use core::arch::x86_64::*;
            
            let size = self.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
            let start = self.header.as_ptr() as *const u8;
            
            // Load start pointer and size into vectors
            let ptr_vector = _mm256_set1_epi64x(start as i64);
            let size_vector = _mm256_set1_epi64x(size as i64);
            
            // Calculate end pointer using SIMD addition
            let end_ptr = _mm256_add_epi64(ptr_vector, size_vector);
            let end = _mm256_extract_epi64(end_ptr, 0) as *const u8;
            
            start..end
        }

        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
        unsafe {
            let size = self.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
            let start = self.header.as_ptr() as *const u8;
            start..(start.add(size))
        }
    }


    #[must_use]
    #[inline(always)]
    pub fn decompose(mut self) -> (Range<NonNull<u8>>, Option<FreeBlock>) {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        unsafe {
            use core::arch::x86_64::*;
            
            let next = self.take_next();
            let header_ptr = self.header.as_ptr();
            
            // Load size using SIMD
            let size = self.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
            let size_vector = _mm256_set1_epi64x(size as i64);
            
            // Calculate end pointer using SIMD addition
            let ptr_vector = _mm256_set1_epi64x(header_ptr as i64);
            let end_ptr = _mm256_add_epi64(ptr_vector, size_vector);
            
            // Extract results
            let start: NonNull<u8> = self.header.cast();
            let end: NonNull<u8> = NonNull::new_unchecked(_mm256_extract_epi64(end_ptr, 0) as *mut u8);
            
            core::mem::forget(self);
            (start..end, next)
        }
        
        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
        {
            let next = self.take_next();
            let range = unsafe {
                let size = self.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
                let start: NonNull<u8> = self.header.cast();
                let end: NonNull<u8> = NonNull::new_unchecked(self.header.as_ptr().add(size) as *mut u8);
                start..end
            };

            core::mem::forget(self);
            (range, next)
        }
    }

    #[inline(always)]
    pub fn next(&self) -> Option<&Self> {
        (&self.header_view().next).into()
    }

    #[inline(always)]
    pub fn next_mut(&mut self) -> Option<&mut Self> {
        unsafe { (&mut self.header_mut().next).into() }
    }

    #[must_use]
    pub fn take_next(&mut self) -> Option<Self> {
        unsafe { (&mut self.header_mut().next).take() }
    }

    #[must_use]
    pub fn replace_next(&mut self, new_next: FreeBlock) -> Option<Self> {
        unsafe { (&mut self.header_mut().next).replace(new_next) }
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.header_view().size.load(core::sync::atomic::Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn header_view(&self) -> &FreeHeader {
        unsafe { self.header.as_ref() }
    }

    #[inline(always)]
    pub unsafe fn header_mut(&mut self) -> &mut FreeHeader {
        self.header.as_mut()
    }

    #[must_use]
    pub fn pop_next(&mut self) -> Option<FreeBlock> {
        let mut next = self.take_next()?;
        if let Some(next_next) = next.take_next() {
            debug_assert!(self.replace_next(next_next).is_none());
        }
        Some(next)
    }
    #[inline(always)]
    pub fn insert(&mut self, block: FreeBlock) {
        let next_next = self.replace_next(block);
        if let Some(next_next_block) = next_next {
            self.next_mut()
                .expect("Just set next, should exist")
                .replace_next(next_next_block);
        }
    }
    #[inline(always)]
    pub fn insert_merge(&mut self, block: FreeBlock) -> usize {
        let this_end = self.as_range().end;
        let other_start = block.as_range().start;
        debug_assert!(block.next().is_none());

        if this_end == other_start {
            let new_size = block.size();
            unsafe {
                // Use relaxed ordering since we have exclusive access
                self.header_mut().size.fetch_add(new_size, Ordering::Relaxed);
                core::mem::forget(block);
            }
            return 1 + self.try_merge_next() as usize;
        }
        
        self.insert(block);
        match self.next_mut() {
            Some(next) => next.try_merge_next() as usize,
            None => 0
        }
    }



    #[inline]
    pub fn try_merge_next(&mut self) -> bool {
        // SAFETY: All pointer operations are within block boundaries
        unsafe {
            // Fast path: check if next block exists
            let Some(next) = self.next() else {
                return false
            };

            // Load all header information at once to minimize cache misses
            let header = self.header_view();
            let current_size = header.size.load(Ordering::Relaxed);
            let current_end = (self.header.as_ptr() as usize).wrapping_add(current_size);
            
            // Check if blocks are adjacent using direct pointer arithmetic
            if current_end == next.header.as_ptr() as usize {
                // Merge blocks in a single operation
                let next_size = next.header_view().size.load(Ordering::Relaxed);
                let header_mut = self.header_mut();
                
                // Update size atomically
                header_mut.size.store(
                    current_size.wrapping_add(next_size),
                    Ordering::Release
                );
                
                // Update next pointer and cleanup
                let mut next_block = header_mut.next.take().unwrap();
                header_mut.next = next_block.take_next();
                core::mem::forget(next_block);
                
                // Try to merge with subsequent block if possible
                if let Some(next_next) = self.next() {
                    if (current_end + next_size) == (next_next.header.as_ptr() as usize) {
                        return self.try_merge_next();
                    }
                }
                
                return true;
            }
            false
        }
    }



    #[inline(always)]
    pub fn split(&mut self, size: usize) -> Range<NonNull<u8>> {
        debug_assert!(size + Self::MIN_SPLIT_SIZE <= self.size());

        // SAFETY: All pointer operations are within block boundaries
        unsafe {
            let header = self.header_mut();
            
            // Use a single atomic operation with Relaxed ordering
            let old_size = header.size.fetch_sub(size, Ordering::Relaxed);
            
            // Optimize pointer arithmetic by doing single cast and minimal operations
            let base = header as *mut FreeHeader as *mut u8;
            
            // Create range using single pointer calculation and wrapping operations
            NonNull::new_unchecked(base.wrapping_add(old_size - size))..
            NonNull::new_unchecked(base.wrapping_add(old_size))
        }
    }




}
impl AsMut<FreeBlock> for FreeBlock {
    fn as_mut(&mut self) -> &mut FreeBlock {
        self
    }
}
unsafe impl Send for FreeBlock {}
unsafe impl Sync for FreeBlock {}