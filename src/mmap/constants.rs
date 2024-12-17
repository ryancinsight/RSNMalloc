// Protection flags
pub const PROT_READ: u64 = 0x01;
pub const PROT_WRITE: u64 = 0x02;

// Mapping flags
pub const MAP_PRIVATE: u64 = 0x02;

#[cfg(target_os = "macos")]
pub const MAP_ANON: u64 = 0x1000;

#[cfg(target_os = "linux")]
pub const MAP_ANON: u64 = 0x20;

#[cfg(target_os = "windows")]
pub const MAP_ANON: u64 = 0x20;

// System call numbers
#[cfg(target_os = "macos")]
pub(crate) const SYS_MMAP: i64 = 0x2000000 + 197;

#[cfg(target_os = "linux")]
pub(crate) const SYS_MMAP: i64 = 9;

#[cfg(target_os = "linux")]
pub(crate) const SYS_MUNMAP: i64 = 11;

#[cfg(target_os = "linux")]
pub(crate) const SYS_MREMAP: i64 = 25;

// mremap flags
#[cfg(target_os = "linux")]
pub const MREMAP_MAYMOVE: u64 = 1;

// mremap flags
#[cfg(target_os = "windows")]
pub const MREMAP_MAYMOVE: u64 = 1;

#[cfg(target_os = "linux")]
pub const MREMAP_FIXED: u64 = 2;

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub const MAP_ANON: u64 = 0x1000;

#[cfg(target_os = "freebsd")]
pub(crate) const SYS_MMAP: i64 = 477;

#[cfg(target_os = "openbsd")]
pub(crate) const SYS_MMAP: i64 = 197;

#[cfg(target_os = "netbsd")]
pub(crate) const SYS_MMAP: i64 = 197;

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub(crate) const SYS_MUNMAP: i64 = 73;