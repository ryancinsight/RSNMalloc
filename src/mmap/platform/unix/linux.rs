use super::syscall::{syscall_mmap, syscall_munmap, syscall_mremap};
use crate::mmap::constants::*;
use crate::mmap::error::MmapError;

#[inline(always)]
pub(crate) unsafe fn unix_mmap(
    addr: *mut u8,
    len: usize,
    prot: u64,
    flags: u64,
    fd: u64,
    offset: i64,
) -> Result<*mut u8, MmapError> {
    syscall_mmap(SYS_MMAP, addr, len, prot, flags, fd, offset)
}
#[inline(always)]
pub(crate) unsafe fn unix_munmap(addr: *mut u8, len: usize) -> Result<(), MmapError> {
    syscall_munmap(SYS_MUNMAP, addr, len)
}

#[inline(always)]
pub(crate) unsafe fn mremap(
    old_addr: *mut u8,
    old_size: usize,
    new_size: usize,
    flags: u64,
) -> Result<*mut u8, MmapError> {
    syscall_mremap(SYS_MREMAP, old_addr, old_size, new_size, flags)
}
