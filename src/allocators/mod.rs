mod atomic_array;
mod generic_allocator;
mod heap_grower;
mod raw_alloc;
mod toy_heap;
mod unix_allocator;

pub use atomic_array::AtomicArray;
pub use generic_allocator::GenericAllocator;
pub use raw_alloc::RawAlloc;
pub use heap_grower::{HeapGrower, EnhancedHeapGrower};
pub use toy_heap::{ToyHeap, ToyHeapOverflowError};
pub use unix_allocator::UnixAllocator;

pub fn round_up(value: usize, increment: usize) -> usize {
    (value + increment - 1) / increment * increment
}
