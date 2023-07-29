use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use nix::TimeVal;
use riscv::register::scause::Scause;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    consts::{PAGE_SIZE, TRAP_CONTEXT_BASE, USER_STACK_SIZE},
    fs::file::File,
    mm::{
        memory_set::{MapPermission, VmAreaType},
        PhysPageNum, VirtAddr,
    },
    trap::TrapContext,
};

use super::{
    kernel_stack::{kstack_alloc, KernelStack},
    ProcessControlBlock, SigMask, SigSet, TaskContext,
};

pub const FD_LIMIT: usize = 1024;

pub struct TaskControlBlock {
    /// 应用内核栈
    pub kstack: KernelStack,

    // immutable
    pub process: Weak<ProcessControlBlock>,
    // mutable
    inner: RwLock<TaskControlBlockInner>,
}

pub struct CloneInfo {
    pub clear_child_tid: usize,
    pub set_child_tid: usize,
    pub tls: usize,
}

impl CloneInfo {
    pub fn empty() -> Self {
        Self {
            clear_child_tid: 0,
            set_child_tid: 0,
            tls: 0,
        }
    }
}

pub struct TimeInfo {
    pub utime: TimeVal,
    pub stime: TimeVal,
    pub last_enter_umode_time: TimeVal,
    pub last_enter_smode_time: TimeVal,
}

impl TimeInfo {
    pub fn empty() -> Self {
        Self {
            utime: TimeVal { sec: 0, usec: 0 },
            stime: TimeVal { sec: 0, usec: 0 },
            last_enter_umode_time: TimeVal { sec: 0, usec: 0 },
            last_enter_smode_time: TimeVal { sec: 0, usec: 0 },
        }
    }
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    // pub clone_info: CloneInfo,
    pub time_info: TimeInfo,
    pub pending_signals: SigSet,
    pub sigmask: SigMask,
    pub trap_cause: Option<Scause>,

    pub clear_child_tid: Option<usize>,
    pub exit_code: Option<i32>,
}

pub type FDTable = Vec<Option<Arc<dyn File>>>;

impl TaskControlBlockInner {
    pub fn trap_cx_mut(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.as_mut()
    }

    fn status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.status() == TaskStatus::Zombie
    }

    pub fn add_utime(&mut self, new_time: TimeVal) {
        self.time_info.utime = self.time_info.utime + new_time;
    }

    pub fn add_stime(&mut self, new_time: TimeVal) {
        self.time_info.stime = self.time_info.stime + new_time;
    }

    pub fn set_last_enter_umode(&mut self, new_time: TimeVal) {
        self.time_info.last_enter_umode_time = new_time;
    }

    pub fn set_last_enter_smode(&mut self, new_time: TimeVal) {
        self.time_info.last_enter_smode_time = new_time;
    }

    pub fn tid(&self) -> usize {
        self.res.as_ref().unwrap().tid
    }
}

impl TaskControlBlock {
    pub fn inner_mut(&self) -> RwLockWriteGuard<'_, TaskControlBlockInner> {
        self.inner.write()
    }

    pub fn inner_ref(&self) -> RwLockReadGuard<'_, TaskControlBlockInner> {
        self.inner.read()
    }

    pub fn is_main_thread(&self) -> bool {
        self.inner_ref().tid() == 0
    }

    // TODO sigmask
    pub fn new(process: Arc<ProcessControlBlock>, ustack_top: usize, alloc_user_res: bool) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), ustack_top, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kstack = kstack_alloc();
        let kstack_top = kstack.top();

        Self {
            process: Arc::downgrade(&process),
            kstack,
            inner: {
                RwLock::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: TaskContext::readied_for_switching(kstack_top),
                    task_status: TaskStatus::Ready,
                    exit_code: None,
                    clear_child_tid: None,
                    time_info: TimeInfo::empty(),
                    sigmask: SigMask::empty(),
                    pending_signals: SigSet::empty(),
                    trap_cause: None,
                })
            },
        }
    }
}

/// 任务状态枚举
///
/// |状态|描述|
/// |--|--|
/// |`Ready`|准备运行|
/// |`Running`|正在运行|
/// |`Zombie`|僵尸态|
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,    // 准备运行
    Running,  // 正在运行
    Blocking, // 阻塞态
    Hanging,  // 挂起态
    Zombie,   // 僵尸态
}

/*********************** TID  ************************/

pub struct RecycleAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl RecycleAllocator {
    pub fn new() -> Self {
        RecycleAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.iter().any(|i| *i == id),
            "id {} has been deallocated!",
            id
        );
        self.recycled.push(id);
    }
}

/// Task User Resource
pub struct TaskUserRes {
    pub tid: usize,
    pub ustack_base: usize,
    pub process: Weak<ProcessControlBlock>,
}

fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

fn ustack_bottom_from_tid(ustack_base: usize, tid: usize) -> usize {
    ustack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}

impl TaskUserRes {
    pub fn new(process: Arc<ProcessControlBlock>, ustack_top: usize, alloc_user_res: bool) -> Self {
        let tid = process.inner_mut().alloc_tid();
        let task_user_res = Self {
            tid,
            ustack_base: ustack_top,
            process: Arc::downgrade(&process),
        };
        if alloc_user_res {
            task_user_res.alloc_user_res();
        }
        task_user_res
    }

    pub fn alloc_user_res(&self) {
        let process = self.process.upgrade().unwrap();
        // alloc user stack
        let ustack_bottom = ustack_bottom_from_tid(self.ustack_base, self.tid);
        let ustack_top = ustack_bottom + USER_STACK_SIZE;

        let mut memory_set = process.memory_set.write();
        memory_set.insert_framed_area(
            ustack_bottom.into(),
            ustack_top.into(),
            MapPermission::R | MapPermission::W | MapPermission::U,
            VmAreaType::UserStack,
        );
        // alloc trap_cx
        let trap_cx_bottom = trap_cx_bottom_from_tid(self.tid);
        let trap_cx_top = trap_cx_bottom + PAGE_SIZE;
        memory_set.insert_framed_area(
            trap_cx_bottom.into(),
            trap_cx_top.into(),
            MapPermission::R | MapPermission::W,
            VmAreaType::TrapContext,
        );
    }

    fn dealloc_user_res(&self) {
        // dealloc tid
        let process = self.process.upgrade().unwrap();
        let mut memory_set = process.memory_set.write();
        // dealloc ustack manually
        let ustack_bottom_va: VirtAddr = ustack_bottom_from_tid(self.ustack_base, self.tid).into();
        memory_set.remove_area_with_start_vpn(ustack_bottom_va.into());
        // dealloc trap_cx manually
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        memory_set.remove_area_with_start_vpn(trap_cx_bottom_va.into());
        drop(memory_set)
    }

    #[allow(unused)]
    pub fn alloc_tid(&mut self) {
        self.tid = self.process.upgrade().unwrap().inner_mut().alloc_tid();
    }

    pub fn dealloc_tid(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_mut();
        process_inner.dealloc_tid(self.tid);
    }

    pub fn trap_cx_user_va(&self) -> usize {
        trap_cx_bottom_from_tid(self.tid)
    }

    pub fn trap_cx_ppn(&self) -> PhysPageNum {
        let process = self.process.upgrade().unwrap();
        let memory_set = process.memory_set.read();
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        memory_set
            .translate(trap_cx_bottom_va.into())
            .unwrap()
            .ppn()
    }

    pub fn ustack_base(&self) -> usize {
        self.ustack_base
    }
    pub fn ustack_top(&self) -> usize {
        ustack_bottom_from_tid(self.ustack_base, self.tid) + USER_STACK_SIZE
    }
}

impl Drop for TaskUserRes {
    fn drop(&mut self) {
        self.dealloc_tid();
        self.dealloc_user_res();
    }
}
