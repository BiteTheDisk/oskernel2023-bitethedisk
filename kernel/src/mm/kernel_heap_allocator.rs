use buddy_system_allocator::LockedHeap;

use crate::consts::KERNEL_HEAP_SIZE;

// 通过 `global_allocator` 注解将 HEAP_ALLOCATOR 标记为 Rust 的内存分配器
// Rust 的相关数据结构，如 Vec, BTreeMap 等，依赖于该分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

// 用于处理动态内存分配失败的情形，直接 panic
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:#x?}", layout);
}

// 给全局分配器用于分配的一块内存，位于内核的 .bss 段中
static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(KERNEL_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}
pub fn heap_usage() {
    let usage_actual = HEAP_ALLOCATOR.lock().stats_alloc_actual();
    let usage_all = HEAP_ALLOCATOR.lock().stats_total_bytes();
    println!("[kernel] HEAP USAGE:{:?} {:?}", usage_actual, usage_all);
}
