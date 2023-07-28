//! processor 提供了一系列的抽象
//!
//! CPU 被抽象为 [`cpu::Cpu`] 对象，指代一个实际的具有 id 的物理 CPU
//!
//! [`Processor`] 是一个物理上的计算单元，和一个具体的 [`cpu::Cpu`] 绑定，
//! 它事实上负责进程 [`TaskControlBlock`] 的运行和进程上下文 [`TaskContext`] 的切换

pub mod processor;
pub mod schedule;

use alloc::sync::Arc;
pub use processor::*;
pub use schedule::*;

use crate::trap::TrapContext;

use super::{process::ProcessControlBlock, switch::__switch, task::TaskControlBlock, TaskContext};

/// 从全局变量 `PROCESSOR` 中取出当前正在执行的任务
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().take_current()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的任务控制块的引用计数的一份拷贝
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    acquire_processor().current().clone()
}

#[inline(always)]
pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().unwrap().process.upgrade().unwrap()
}

/// 从全局变量 `PROCESSOR` 中取出当前正在执行任务的用户地址空间 token
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let memory_set = process.memory_set.read();
    let token = memory_set.token();
    drop(memory_set);

    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().inner_mut().trap_cx_mut()
}

/// 换到 idle 控制流并开启新一轮的任务调度
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = acquire_processor();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);

    unsafe { __switch(switched_task_cx_ptr, idle_task_cx_ptr) }
}

pub fn current_trap_cx_user_va() -> usize {
    current_task()
        .unwrap()
        .inner_ref()
        .res
        .as_ref()
        .unwrap()
        .trap_cx_user_va()
}
