//! This is a very minimal example to show using the HeapGrower functions.

use basic_allocator::allocators::HeapGrower;
use basic_allocator::allocators::EnhancedHeapGrower;

fn main() {
    {
        // LibcHeapGrower uses libc to call mmap
        println!("Using libc");
        let mut lhg = EnhancedHeapGrower::default();
        let (p, sz) = unsafe { lhg.grow_heap(8).unwrap() };
        println!("Returned: ({:p}={}, {})", p, p as i64, sz);
    }
}
