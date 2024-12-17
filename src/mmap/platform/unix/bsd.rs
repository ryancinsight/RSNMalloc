use super::syscall::{syscall_mmap, syscall_munmap};
use crate::mmap::constants::*;
use crate::mmap::error::MmapError;

pub(crate) unsafe fn mmap(
    addr: *mut u8,
    len: usize,
    prot: u64,
    flags: u64,
    fd: u64,
    offset: i64,
) -> Result<*mut u8, MmapError> {
    syscall_mmap(SYS_MMAP, addr, len, prot, flags, fd, offset)
}

pub(crate) unsafe fn munmap(addr: *mut u8, len: usize) -> Result<(), MmapError> {
    syscall_munmap(SYS_MUNMAP, addr, len)
}

pub(crate) unsafe fn mremap(
    old_addr: *mut u8,
    old_size: usize,
    new_size: usize,
    _flags: u64,
) -> Result<*mut u8, MmapError> {
    let new_addr = unix_mmap(
        core::ptr::null_mut(),
        new_size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANON,
        0,
        0,
    )?;

    let copy_size = if new_size > old_size { old_size } else { new_size };
    core::ptr::copy_nonoverlapping(old_addr, new_addr, copy_size);

    unix_munmap(old_addr, old_size)?;

    Ok(new_addr)
}
