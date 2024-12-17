use super::*;
use crate::mmap::platform::mremap;
#[test]
fn test_mmap_success() {
    unsafe {
        let result = mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            0,
            0,
        );
        assert!(result.is_ok(), "Expected mmap to succeed");
        let ptr = result.unwrap();
        let unmap_result = munmap(ptr, 4096);
        assert!(unmap_result.is_ok(), "Expected munmap to succeed");
    }
}

#[test]
fn test_mmap_failure() {
    unsafe {
        let result = mmap(
            core::ptr::null_mut(),
            usize::MAX,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            0,
            0,
        );
        assert!(result.is_err(), "Expected mmap to fail");
    }
}

#[test]
fn test_mremap() {
    unsafe {
        let ptr = mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            0,
            0,
        )
        .unwrap();
        let new_ptr = mremap(ptr, 4096, 8192, MREMAP_MAYMOVE).unwrap();
        munmap(new_ptr, 8192).unwrap();
    }
}