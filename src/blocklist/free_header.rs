use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use super::free_block::FreeBlock;

#[derive(Debug)]
#[repr(C, align(16))]
pub struct FreeHeader {
    pub(crate) next: Option<FreeBlock>,
    pub(crate) size: AtomicUsize,
}

const HEADER_SIZE: usize = 16;

impl FreeHeader {
    #[inline(always)]
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn from_raw(
        ptr: NonNull<u8>,
        next: Option<FreeBlock>,
        size: usize,
    ) -> NonNull<FreeHeader> {
        let header = FreeHeader {
            next,
            size: AtomicUsize::new(size),
        };
        let raw_ptr: NonNull<FreeHeader> = ptr.cast();
        core::ptr::write(ptr.as_ptr() as *mut FreeHeader, header);
        raw_ptr
    }

    #[inline(always)]
    pub fn get_size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn set_size(&self, new_size: usize) {
        self.size.store(new_size, Ordering::Release);
    }
}

unsafe impl Send for FreeHeader {}
unsafe impl Sync for FreeHeader {}

pub const fn header_size() -> usize {
    HEADER_SIZE
}
