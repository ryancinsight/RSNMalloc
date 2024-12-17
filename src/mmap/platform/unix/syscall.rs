use core::arch::asm;
use crate::mmap::error::MmapError;

#[inline(always)]
pub(crate) unsafe fn syscall_mmap(
    syscall_num: i64,
    addr: *mut u8,
    len: usize,
    prot: u64,
    flags: u64,
    fd: u64,
    offset: i64,
) -> Result<*mut u8, MmapError> {
    let mut out_addr: i64;
    
    asm!(
        "syscall",
        in("rax") syscall_num,
        in("rdi") addr as i64,
        in("rsi") len,
        in("rdx") prot,
        in("r10") flags,
        in("r8") fd,
        in("r9") offset,
        lateout("rax") out_addr,
        options(nostack),
    );

    if out_addr < 0 {
        return Err(MmapError {
            code: -out_addr,
            message: "mmap syscall failed",
        });
    }

    Ok(out_addr as *mut u8)
}

#[inline(always)]
pub(crate) unsafe fn syscall_munmap(
    syscall_num: i64,
    addr: *mut u8,
    len: usize,
) -> Result<(), MmapError> {
    let result: i64;
    
    asm!(
        "syscall",
        inout("rax") syscall_num => result,
        in("rdi") addr as i64,
        in("rsi") len as i64,
        options(nostack),
    );

    if result != 0 {
        return Err(MmapError {
            code: result,
            message: "munmap syscall failed",
        });
    }

    Ok(())
}

#[inline(always)]
pub(crate) unsafe fn syscall_mremap(
    syscall_num: i64,
    old_addr: *mut u8,
    old_size: usize,
    new_size: usize,
    flags: u64,
) -> Result<*mut u8, MmapError> {
    let mut out_addr: i64;
    
    asm!(
        "syscall",
        in("rax") syscall_num,
        in("rdi") old_addr as i64,
        in("rsi") old_size,
        in("rdx") new_size,
        in("r10") flags,
        lateout("rax") out_addr,
        options(nostack),
    );

    if out_addr < 0 {
        return Err(MmapError {
            code: -out_addr,
            message: "mremap syscall failed",
        });
    }

    Ok(out_addr as *mut u8)
}
