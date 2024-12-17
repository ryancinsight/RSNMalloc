use core::fmt::{self, Display};
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default, Debug)]
pub struct Stats {
    pub length: AtomicUsize,
    pub size: AtomicUsize,
}

impl Stats {
    #[inline(always)]
    pub fn add_block(&self, size: usize) {
        self.length.fetch_add(1, Ordering::Release);
        self.size.fetch_add(size, Ordering::Release);
    }

    #[inline(always)]
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.length.load(Ordering::Acquire),
            self.size.load(Ordering::Acquire)
        )
    }
}

impl Display for Stats {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (length, size) = self.get_stats();
        write!(
            f,
            "{:#?} empty blocks in allocator, total size {:#?} bytes",
            length, size
        )
    }
}