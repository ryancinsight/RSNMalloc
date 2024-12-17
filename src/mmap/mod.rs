#![cfg(not(feature = "use_libc"))]

mod constants;
mod error;
mod platform;
#[cfg(test)]
mod tests;

pub use constants::*;
pub use error::MmapError;
pub use platform::{mmap, munmap,mremap};