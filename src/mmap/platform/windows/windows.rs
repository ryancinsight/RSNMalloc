use crate::mmap::error::MmapError;

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_READWRITE: u32 = 0x04;

#[link(name = "kernel32")]
extern "system" {
    #[inline(always)]
    fn VirtualAlloc(
        lpAddress: *mut core::ffi::c_void,
        dwSize: usize,
        flAllocationType: u32,
        flProtect: u32
    ) -> *mut core::ffi::c_void;
    #[inline(always)]
    fn VirtualFree(
        lpAddress: *mut core::ffi::c_void,
        dwSize: usize,
        dwFreeType: u32
    ) -> i32;
}

#[inline(always)]
pub(crate) unsafe fn windows_mmap(
    addr: *mut u8,
    len: usize,
    _prot: u64,
    _flags: u64,
    _fd: u64,
    _offset: i64,
) -> Result<*mut u8, MmapError> {
    let ptr = VirtualAlloc(
        addr as *mut core::ffi::c_void,
        len,
        MEM_RESERVE | MEM_COMMIT,
        PAGE_READWRITE,
    );
    
    if ptr.is_null() {
        return Err(MmapError {
            code: -1,
            message: "VirtualAlloc failed",
        });
    }
    
    Ok(ptr as *mut u8)
}

#[inline(always)]
pub(crate) unsafe fn windows_munmap(addr: *mut u8) -> Result<(), MmapError> {
    let success = VirtualFree(addr as *mut core::ffi::c_void, 0, MEM_RELEASE);
    if success == 0 {
        return Err(MmapError {
            code: -1,
            message: "VirtualFree failed",
        });
    }
    
    Ok(())
}
#[inline(always)]
pub(crate) unsafe fn windows_mremap(
    old_address: *mut u8,
    old_size: usize,
    new_size: usize,
    _flags: u64,
) -> Result<*mut u8, MmapError> {
    // Allocate new memory region
    let new_ptr = windows_mmap(
        core::ptr::null_mut(),
        new_size,
        PAGE_READWRITE as u64,
        (MEM_RESERVE | MEM_COMMIT) as u64,
        0,
        0
    )?;

    // Copy data from old to new location
    if old_size > 0 {
        core::ptr::copy_nonoverlapping(
            old_address,
            new_ptr,
            core::cmp::min(old_size, new_size)
        );
    }

    // Free old memory
    windows_munmap(old_address)?;

    Ok(new_ptr)
}
