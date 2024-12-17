use core::fmt;
use core::ops::Range;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};
use core::sync::atomic::AtomicUsize;
use super::free_block::FreeBlock;
use super::validity::Validity;
use super::stats::Stats;
use crate::relation::Relation;

#[derive(Debug)]
#[repr(align(64))]
pub struct BlockList {
    first: Option<FreeBlock>,
    merged: AtomicBool,
    length: AtomicUsize,
}

impl Default for BlockList {
    #[inline(always)]
    fn default() -> Self {
        BlockList {
            first: None,
            merged: AtomicBool::new(true),
            length: AtomicUsize::new(0),
        }
    }
}


#[derive(Debug)]
#[repr(transparent)]
pub struct BlockIter<'list> {
    next: Option<&'list FreeBlock>,
}

impl<'list> Iterator for BlockIter<'list> {
    type Item = &'list FreeBlock;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = current.next();
        Some(current)
    }
}

impl<'list> IntoIterator for &'list BlockList {
    type Item = &'list FreeBlock;
    type IntoIter = BlockIter<'list>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        BlockIter {
            next: self.first.as_ref(),
        }
    }
}

impl fmt::Display for BlockList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockList(")?;
        let mut start = true;
        for block in self {
            if !start {
                write!(f, ", ")?;
            } else {
                start = false;
            }
            write!(f, "FreeBlock({:?}, {})", block.header_view(), block.size())?;
        }
        write!(f, ")")
    }
}

pub enum ApplyState<C, R> {
    Continue(C),
    Finished(R),
    Fail(C),
}

impl<C, R> ApplyState<C, R> {
	#[inline(always)]
    pub fn into_result(self) -> Option<R> {
        match self {
            ApplyState::Finished(result) => Some(result),
            _ => None,
        }
    }
}

impl BlockList {
    // Cache line size optimization
    const BATCH_SIZE: usize = 8;
    const CACHE_LINE_SIZE: usize = 64;
    #[inline(always)]
    pub fn header_size() -> usize {
        super::free_header::header_size()
    }
    #[cfg(test)]
    pub fn get_total_memory(&self) -> usize {
        self.iter().fold(0, |acc, block| acc + block.size())
    }
    #[inline(always)]
    unsafe fn process_batch(&self, blocks: &[FreeBlock]) -> Option<NonNull<FreeBlock>> {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            use core::arch::x86_64::*;
            if blocks.len() >= 4 {
                let sizes = _mm256_setr_epi64x(
                    blocks[0].size() as i64,
                    blocks[1].size() as i64,
                    blocks[2].size() as i64,
                    blocks[3].size() as i64
                );
                // Process sizes in parallel
                let aligned_sizes = _mm256_add_epi64(
                    sizes,
                    _mm256_set1_epi64x(15)
                );
                // Find best fit
                let mask = _mm256_movemask_pd(_mm256_castsi256_pd(aligned_sizes));
                if mask != 0 {
                    let idx = mask.trailing_zeros() as usize;
                    return Some(blocks[idx].header);
                }
            }
        }
        None
    }
    
    #[inline(always)]
    pub unsafe fn add_block(&mut self, ptr: NonNull<u8>, size: usize) {
        let new_block = FreeBlock::from_raw(ptr, None, size);
        match self.first.take() {
            None => {
                self.first = Some(new_block);
                self.length.fetch_add(1, Ordering::Relaxed);
            }
            Some(first) => {
                match new_block.relation(&first) {
                    Relation::Before | Relation::AdjacentBefore => {
                        let mut block = new_block;
                        block.replace_next(first);
                        block.try_merge_next();
                        self.first = Some(block);
                        self.try_merge_all();
                    }
                    _ => {
                        self.first = Some(first);
                        self.merge_block(new_block);
                        self.try_merge_all();
                    }
                }
            }
        }
    }
    #[inline(always)]
    unsafe fn try_merge_all(&mut self) {
        while self.merged.swap(false, Ordering::Relaxed) {
            let mut current = self.first.as_mut(); // Start from the first block

            while let Some(block) = current {
                // Safely get `next_block` without holding multiple borrows on `block`.
                let next_block_option = block.next_mut();
                if next_block_option.is_none() {
                    break; // No next block, end the loop
                }

                // Take ownership of the next block safely.
                let mut next_block = block.take_next().unwrap();

                // Compare ranges to check for adjacency.
                let block_range_end = block.as_range().end;
                let next_range_start = next_block.as_range().start;

                if block_range_end == next_range_start {
                    let next_size = next_block.size();

                    // Update `block`'s size and next pointer.
                    let block_header = block.header_mut();
                    block_header.size.fetch_add(next_size, Ordering::Release);
                    block_header.next = next_block.take_next(); // Safe transfer of ownership

                    self.merged.store(true, Ordering::Relaxed);
                    self.length.fetch_sub(1, Ordering::Relaxed);

                    // Restart merging from the first block.
                    current = self.first.as_mut();
                } else {
                    // Reinsert `next_block` since it was taken out.
                    block.replace_next(next_block);
                    current = block.next_mut();
                }
            }
        }
    }







    #[inline(always)]
    unsafe fn merge_block(&mut self, block: FreeBlock) {
        self.apply(block, |prev, new_block| {
            if let Some(next) = prev.next() {
                match new_block.relation(next) {
                    Relation::Before | Relation::AdjacentBefore => {
                        prev.insert_merge(new_block);
                        ApplyState::Finished(())
                    }
                    Relation::AdjacentAfter | Relation::After if prev.try_merge_next() => {
                        ApplyState::Finished(())
                    }
                    _ => ApplyState::Continue(new_block),
                }
            } else {
                prev.insert_merge(new_block);
                ApplyState::Finished(())
            }
        });
    }


    
    #[inline(always)]
    pub fn iter(&self) -> BlockIter {
        BlockIter {
            next: self.first.as_ref(),
        }
    }
    #[inline(always)]
    pub fn find_adjacent(&self, ptr: *mut u8, size: usize) -> Option<Range<NonNull<u8>>> {
        unsafe {
            // Get first block and target address
            let mut current = self.first.as_ref()?;
            let target_addr = ptr.add(size);
            
            // Cache header pointer for current block
            let mut header_ptr = current.header.as_ptr();
            
            loop {
                let block_start = header_ptr as *mut u8;
                
                if block_start >= target_addr {
                    // Create range directly from cached values
                    let block_size = (*header_ptr).size.load(Ordering::Relaxed);
                    return Some(
                        NonNull::new_unchecked(block_start)..
                        NonNull::new_unchecked(block_start.add(block_size))
                    );
                }
                
                // Update pointers directly without match
                current = current.next()?;
                header_ptr = current.header.as_ptr();
            }
        }
    }







    #[inline]
    pub fn apply<C, R, F: FnMut(&mut FreeBlock, C) -> ApplyState<C, R>>(
        &mut self,
        start: C,
        mut pred: F,
    ) -> ApplyState<C, R> {
        let mut next = self.first.as_mut();
        let mut state = start;
        
        while let Some(block) = next.take() {
            state = match pred(block, state) {
                ApplyState::Continue(c) => c,
                ApplyState::Finished(r) => return ApplyState::Finished(r),
                ApplyState::Fail(c) => return ApplyState::Fail(c),
            };
            next = block.next_mut();
        }
        
        ApplyState::Continue(state)
    }
    #[inline]
    pub fn stats(&self) -> (Validity, Stats) {
        let validity = Validity::default();
        let stats = Stats::default();
        let mut previous: Option<&FreeBlock> = None;

        for block in self.iter() {
            if let Some(prev) = previous {
                match prev.relation(block) {
                    Relation::Before => {},
                    Relation::AdjacentBefore => validity.record_adjacent(),
                    Relation::Overlapping => validity.record_overlap(),
                    Relation::AdjacentAfter => {
                        validity.record_out_of_order();
                        validity.record_adjacent();
                    },
                    Relation::After => validity.record_out_of_order(),
                }
            }
            
            stats.add_block(block.size());
            previous = Some(block);
        }

        (validity, stats)
    }

    #[inline]
    pub fn pop_size(&mut self, size: usize) -> Option<Range<NonNull<u8>>> {
        const HEADER_SIZE: usize = super::free_header::header_size();
        let min_size = size.wrapping_add(HEADER_SIZE);

        // Fast path: check first block
        if let Some(first) = self.first.as_mut() {
            let first_size = unsafe { first.header_view().size.load(core::sync::atomic::Ordering::Relaxed) };
            if first_size >= size {
                unsafe {
                    if first_size == size {
                        let range = NonNull::new_unchecked(first.header.as_ptr() as *mut u8)..
                                    NonNull::new_unchecked(first.header.as_ptr().add(size) as *mut u8);
                        self.first = first.take_next();
                        return Some(range);
                    }
                    return Some(first.split(size));
                }
            }
        }

        self.apply((), |previous, ()| unsafe {
            let Some(next) = previous.next_mut() else {
                return ApplyState::Continue(());
            };
            
            let next_size = next.header_view().size.load(core::sync::atomic::Ordering::Relaxed);
            
            if next_size >= size {
                ApplyState::Finished(
                    if next_size == size {
                        // Exact match: direct range creation
                        let ptr = next.header.as_ptr();
                        previous.header_mut().next = next.take_next();
                        NonNull::new_unchecked(ptr as *mut u8)..
                        NonNull::new_unchecked(ptr.add(size) as *mut u8)
                    } else {
                        // Larger block: split
                        next.split(size)
                    }
                )
            } else {
                ApplyState::Continue(())
            }
        })
        .into_result()


    }


    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for BlockList {
    #[inline]
    fn drop(&mut self) {
        let mut current = self.first.take();
        while let Some(mut block) = current {
            current = block.take_next();
            let size = block.size();
            unsafe {
                core::ptr::write_bytes(block.header_view() as *const _ as *mut u8, 0, size);
            }
        }
    }
}