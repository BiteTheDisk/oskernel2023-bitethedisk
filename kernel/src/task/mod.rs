pub mod context; // 任务上下文模块
mod initproc;
mod kernel_stack;
mod manager; // 进程管理器
mod pid; // 进程标识符模块
pub mod processor; // 处理器管理模块
mod signals;
mod switch; // 任务上下文切换模块
pub mod task;

use alloc::sync::Arc;
use fat32::sync_all;
use manager::remove_from_pid2task;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::{add_task, check_hanging, pid2task, unblock_task};
pub use pid::{pid_alloc, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, schedule::*, take_current_task,
};
pub use signals::*;
pub use task::FD_LIMIT;

use self::{
    initproc::INITPROC,
    manager::block_task,
    processor::{acquire_processor, schedule},
};

/// 将当前任务置为就绪态，放回到进程管理器中的就绪队列中，重新选择一个进程运行
pub fn suspend_current_and_run_next() -> isize {
    // 取出当前正在执行的任务
    let task_cp = current_task().unwrap();
    let mut task_inner = task_cp.write();
    if task_inner.signals.contains(SignalFlags::SIGKILL) {
        let exit_code = task_inner.exit_code;
        drop(task_inner);
        drop(task_cp);
        exit_current_and_run_next(exit_code);
        return 0;
    }
    let task = take_current_task().unwrap();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 修改其进程控制块内的状态为就绪状态
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    // 将进程加入进程管理器中的就绪队列
    add_task(task);

    // 开启一轮新的调度
    schedule(task_cx_ptr);

    0
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // println!("[KERNEL] pid:{} exited", current_task().unwrap().pid.0);

    // 获取访问权限，修改进程状态
    let task = take_current_task().unwrap();
    remove_from_pid2task(task.pid());
    let mut inner = task.write();
    inner.task_status = TaskStatus::Zombie; // 后续才能被父进程在 waitpid 系统调用的时候回收
                                            // 记录退出码，后续父进程在 waitpid 的时候可以收集
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    if task.pid() == 0 {
        sync_all();
        panic!("initproc return!");
    }

    // 将这个进程的子进程转移到 initproc 进程的子进程中
    let mut initproc_inner = INITPROC.write();
    for child in inner.children.iter() {
        child.write().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone()); // 引用计数 -1
    }
    drop(initproc_inner);

    // 引用计数 +1
    // 对于当前进程占用的资源进行早期回收
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);

    // 使用全0的上下文填充换出上下文，开启新一轮进程调度
    let mut _unused = TaskContext::empty();
    schedule(&mut _unused as *mut _);
}

pub fn hanging_current_and_run_next(sleep_time: usize, duration: usize) {
    let task = current_task().unwrap();
    let mut inner = task.write();
    let current_cx_ptr = &mut inner.task_cx as *mut TaskContext;
    inner.task_status = TaskStatus::Hanging;
    drop(inner);
    drop(task);
    acquire_processor().hang_current(sleep_time, duration);
    schedule(current_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.write();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Blocking;
    block_task(task.clone());

    drop(task_inner);
    drop(task);

    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// 将初始进程 `initproc` 加入任务管理器
pub fn add_initproc() {
    add_task(INITPROC.clone());
}
