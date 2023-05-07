//!
//! Task kernel stack 应用内核栈
//!
use crate::{
    consts::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{kernel_vmm::KERNEL_VMM, MapPermission, VirtAddr},
};

use super::PidHandle;

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

/// 进程内核栈
pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// 从一个已分配的进程标识符中对应生成一个内核栈
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_VMM.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        KernelStack { pid: pid_handle.0 }
    }

    /// 获取当前应用内核栈顶在内核地址空间中的地址(这地址仅与app_id有关)
    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_VMM
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}