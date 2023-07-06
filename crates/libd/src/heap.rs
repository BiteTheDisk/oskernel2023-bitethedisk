#![allow(unused)]

use buddy_system_allocator::LockedHeap;

pub(crate) const USER_HEAP_SIZE: usize = 65536;
pub(crate) static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
pub(crate) static HEAP: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout: {:?}", layout);
}

pub unsafe fn init() -> Result<(), ()> {
    HEAP.lock()
        .init(HEAP_SPACE.as_ptr() as usize, HEAP_SPACE.len());
    Ok(())
}
