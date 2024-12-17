use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

#[cfg(feature = "use_libc")]
use errno::Errno;

#[cfg(not(feature = "use_libc"))]
use crate::mmap::{self, MmapError};

pub trait HeapGrower {
    type Err;
    unsafe fn grow_heap(&mut self, size: usize) -> Result<(*mut u8, usize), Self::Err>;
}

#[derive(Default)]
pub struct EnhancedHeapGrower {
    // Enhanced tracking mechanisms
    pages: AtomicUsize,
    growths: AtomicUsize,
    base: AtomicPtr<u8>,
    total_allocated: AtomicUsize,
    peak_allocation: AtomicUsize,
    allocation_attempts: AtomicUsize,
}

impl EnhancedHeapGrower {
    #[inline(always)]
    const fn round_up(value: usize, increment: usize) -> usize {
        (value + increment - 1) & !(increment - 1)
    }

    #[inline(always)]
    fn get_page_size() -> usize {
        // Standard page size for most architectures
        const DEFAULT_PAGE_SIZE: usize = 4096;

        // Architecture-specific page size detection
        #[cfg(target_arch = "x86_64")]
        {
            return DEFAULT_PAGE_SIZE;
        }

        #[cfg(target_arch = "aarch64")]
        {
            return DEFAULT_PAGE_SIZE;
        }

        #[cfg(target_arch = "arm")]
        {
            return DEFAULT_PAGE_SIZE;
        }

        #[cfg(target_os = "windows")]
        {
            // Ultra-low-level Windows page size detection
            unsafe {
                // Direct system call approach (no libc)
                const SYSTEM_INFO_OFFSET: usize = 44; // Offset to dwPageSize in SYSTEM_INFO structure
                let mut system_info = core::mem::zeroed::<[u8; 64]>();
                
                // Windows-specific system call to get system information
                #[cfg(target_arch = "x86_64")]
                core::arch::asm!(
                    "syscall",
                    in("rax") 0x34, // GetSystemInfo syscall number
                    in("rcx") system_info.as_mut_ptr(),
                    options(nostack)
                );

                // Extract page size from the system info structure
                return *(system_info.as_ptr().add(SYSTEM_INFO_OFFSET) as *const usize);
            }
        }

        #[cfg(unix)]
        {
            // Low-level Unix page size detection
            unsafe {
                // Direct system call for page size
                let mut page_size: usize = 0;
                
                #[cfg(target_arch = "x86_64")]
                core::arch::asm!(
                    "syscall",
                    in("rax") 0xc0, // sysconf syscall
                    in("rdi") 30,   // _SC_PAGESIZE constant
                    out("rcx") page_size,
                    options(nostack)
                );

                return page_size.max(DEFAULT_PAGE_SIZE);
            }
        }

        // Fallback to default page size
        DEFAULT_PAGE_SIZE
    }


}

impl HeapGrower for EnhancedHeapGrower {
    #[cfg(not(feature = "use_libc"))]
    type Err = MmapError;
    
    #[cfg(feature = "use_libc")]
    type Err = Errno;

    unsafe fn grow_heap(&mut self, size: usize) -> Result<(*mut u8, usize), Self::Err> {
        // Increment allocation attempts
        self.allocation_attempts.fetch_add(1, Ordering::Relaxed);

        if size == 0 {
            return Ok((null_mut(), 0));
        }

        let page_size = Self::get_page_size();
        let to_allocate = Self::round_up(size, page_size);

        // Allocation with platform-specific method
        #[cfg(not(feature = "use_libc"))]
        let ptr = mmap::mmap(
            null_mut(),
            to_allocate,
            mmap::PROT_WRITE | mmap::PROT_READ,
            mmap::MAP_ANON | mmap::MAP_PRIVATE,
            u64::MAX,
            0,
        )?;

        #[cfg(feature = "use_libc")]
        let ptr = libc::mmap(
            null_mut(),
            to_allocate,
            libc::PROT_WRITE | libc::PROT_READ,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        );

        // Check allocation success
        #[cfg(feature = "use_libc")]
        if ptr == libc::MAP_FAILED {
            return Err(errno::errno());
        }

        // Update tracking metrics
        let current_total = self.total_allocated.fetch_add(to_allocate, Ordering::Relaxed);
        self.peak_allocation.fetch_max(current_total.wrapping_add(to_allocate), Ordering::Relaxed);
        
        self.pages.fetch_add(to_allocate.wrapping_div(page_size), Ordering::Relaxed);
        self.growths.fetch_add(1, Ordering::Relaxed);
        self.base.store(ptr as *mut u8, Ordering::Relaxed);

        Ok((ptr as *mut u8, to_allocate))
    }
}

impl Drop for EnhancedHeapGrower {
    fn drop(&mut self) {
        unsafe {
            let page_size = EnhancedHeapGrower::get_page_size();
            let pages = self.pages.load(Ordering::Relaxed);
            
            if pages > 0 {
                let size = pages * page_size;
                
                #[cfg(not(feature = "use_libc"))]
                let _ = mmap::munmap(self.base.load(Ordering::Relaxed) as *mut _, size);
                
                #[cfg(feature = "use_libc")]
                libc::munmap(self.base.load(Ordering::Relaxed) as *mut _, size);
            }
        }
    }
}

// Optional: Expose tracking methods
impl EnhancedHeapGrower {
    pub fn total_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }

    pub fn peak_allocation(&self) -> usize {
        self.peak_allocation.load(Ordering::Relaxed)
    }

    pub fn allocation_attempts(&self) -> usize {
        self.allocation_attempts.load(Ordering::Relaxed)
    }
}
