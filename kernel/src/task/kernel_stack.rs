//! 进程内核栈

use spin::RwLock;

use crate::{
    consts::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{kernel_vmm::acquire_kvmm, memory_set::VmAreaType, MapPermission, VirtAddr},
    task::task::RecycleAllocator,
};

lazy_static! {
    static ref KSTACK_ALLOCATOR: RwLock<RecycleAllocator> = RwLock::new(RecycleAllocator::new());
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(kstack_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - kstack_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub struct KernelStack(pub usize);

pub fn kstack_alloc() -> KernelStack {
    let kstack_id = KSTACK_ALLOCATOR.write().alloc();
    let (kstack_bottom, kstack_top) = kernel_stack_position(kstack_id);

    acquire_kvmm().insert_framed_area(
        kstack_bottom.into(),
        kstack_top.into(),
        MapPermission::R | MapPermission::W,
        VmAreaType::KernelStack,
    );
    KernelStack(kstack_id)
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.0);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        acquire_kvmm().remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

impl KernelStack {
    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.0);
        kernel_stack_top
    }
}
