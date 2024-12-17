mod unix;
mod windows;
mod wasm;

use crate::mmap::error::MmapError;

pub unsafe fn mmap(
    addr: *mut u8,
    len: usize,
    prot: u64,
    flags: u64,
    fd: u64,
    offset: i64,
) -> Result<*mut u8, MmapError> {
    #[cfg(unix)]
    return unix::unix_mmap(addr, len, prot, flags, fd, offset);
    #[cfg(windows)]
    return windows::windows_mmap(addr, len, prot, flags, fd, offset);
    #[cfg(target_arch = "wasm32")]
    return wasm::wasm_mmap(addr, len, prot, flags, fd, offset);
}

pub unsafe fn munmap(addr: *mut u8, len: usize) -> Result<(), MmapError> {
    #[cfg(unix)]
    return unix::unix_munmap(addr, len);
    #[cfg(windows)]
    return windows::windows_munmap(addr);
    #[cfg(target_arch = "wasm32")]
    return wasm::wasm_munmap(addr, len);
}

pub unsafe fn mremap(
    old_addr: *mut u8,
    old_size: usize,
    new_size: usize,
    flags: u64,
) -> Result<*mut u8, MmapError> {
    #[cfg(target_os = "linux")]
    return unix::mremap(old_addr, old_size, new_size, flags);
    #[cfg(target_os = "windows")]
    return windows::windows_mremap(old_addr, old_size, new_size, flags);
    #[cfg(target_arch = "wasm32")]
    return wasm::mremap(old_addr, old_size, new_size, flags);
}
