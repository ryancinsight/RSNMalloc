use core::sync::atomic::{AtomicU8, Ordering};
extern crate alloc;
use alloc::vec::Vec;

#[derive(Default)]
pub struct AtomicArray {
    data: Vec<AtomicU8>, // Using Vec for dynamic sizing
}

impl AtomicArray {
    /// Creates a new AtomicArray with the given size
    pub fn new(size: usize) -> Self {
        Self {
            data: (0..size).map(|_| AtomicU8::new(0)).collect(),
        }
    }

    /// Atomically loads a value at the specified index
    pub fn load(&self, index: usize, ordering: Ordering) -> u8 {
        self.data[index].load(ordering)
    }

    /// Atomically stores a value at the specified index
    pub fn store(&self, index: usize, value: u8, ordering: Ordering) {
        self.data[index].store(value, ordering);
    }

    /// Atomically adds a value to the element at the specified index
    pub fn fetch_add(&self, index: usize, value: u8, ordering: Ordering) -> u8 {
        self.data[index].fetch_add(value, ordering)
    }

    /// Returns the length of the array
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns a raw pointer to the underlying data
    pub fn as_ptr(&self) -> *const AtomicU8 {
        self.data.as_ptr()
    }
}
