use alloc::{sync::Arc, vec::Vec};

use crate::{
    consts::SIGNAL_TRAMPOLINE,
    mm::{copyout, translated_mut},
    syscall::futex_wake,
    task::signals::{SAFlags, SigInfo, SignalContext, UContext},
};

use self::{
    context::TaskContext,
    initproc::INITPROC,
    manager::{add_task, block_task, remove_from_pid2process},
    process::ProcessControlBlock,
    processor::{acquire_processor, current_task, schedule, take_current_task},
    signals::{SigMask, SigSet, Signal},
    task::{TaskStatus, TaskUserRes},
};

pub mod context; // 任务上下文模块
pub mod initproc;
pub mod kernel_stack;
pub mod manager; // 进程管理器
pub mod pid; // 进程标识符模块
pub mod process;
pub mod processor; // 处理器管理模块
pub mod signals;
pub mod switch; // 任务上下文切换模块
pub mod task;

/// 将当前任务置为就绪态，放回到进程管理器中的就绪队列中，重新选择一个进程运行
pub fn suspend_current_and_run_next() {
    // 取出当前正在执行的任务
    let task = current_task().unwrap();
    let task_inner = task.inner_ref();
    if task_inner.pending_signals.contains(SigMask::SIGKILL) {
        let exit_code = task_inner.exit_code.unwrap(); // SIGKILL(9)
        drop(task_inner);
        drop(task);
        exit_current_and_run_next(exit_code, false);
        panic!("Shouldn't reach here in `suspend_current_and_run_next`!")
    }
    drop(task_inner);

    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_mut();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 修改其进程控制块内的状态为就绪状态
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    // 将“add_task”延迟到调度完成（即切换到idle控制流）之后
    // 若不这么做，该线程将自己挂到就绪队列，另一cpu核可能趁此机会取出该线程，并进入该线程的内核栈
    // 从而引发错乱。
    // 将进程加入进程管理器中的就绪队列
    add_task(task);

    // 开启一轮新的调度
    schedule(task_cx_ptr);
}

pub fn exit_current_and_run_next(exit_code: i32, is_exit_group: bool) {
    let task = take_current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let mut task_inner = task.inner_mut();
    let tid = task_inner.res.as_ref().unwrap().tid;

    // do futex_wake if clear_child_tid is set
    if let Some(clear_child_tid) = task_inner.clear_child_tid {
        let memory_set = process.memory_set.read();
        *translated_mut(memory_set.token(), clear_child_tid as *mut usize) = 0;
        drop(memory_set);
        futex_wake(clear_child_tid, 1).unwrap();
    }

    // record exit code
    task_inner.exit_code = Some(exit_code);
    // remove user task resource
    task_inner.res = None;
    // here we do not remove the thread since we are still using the kstack
    drop(task_inner);
    drop(task);

    // however, if this is the main thread of current process (tid == 0)
    // the process should terminate at once
    if tid == 0 || is_exit_group {
        let pid = process.get_pid();
        if pid == 0 {
            // initproc
            unimplemented!("initproc exit!")
        }
        remove_from_pid2process(pid);
        let mut process_inner = process.inner_mut();
        // record exit code of main process
        process_inner.exit_code = Some(exit_code);

        // move all child processes under init process
        let mut initproc_inner = INITPROC.inner_mut();
        for child in process_inner.children.iter() {
            child.inner_mut().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
        drop(initproc_inner);

        // deallocate user res (including tid/trap_cx/ustack) of all threads
        // it has to be done before we dealloc the whole memory_set
        // otherwise they will be deallocated twice
        let mut recycle_res = Vec::<TaskUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            let mut task_inner = task.inner_mut();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        // dealloc_tid and dealloc_user_res require access to PCB inner, so we
        // need to collect those user res first, then release process_inner
        // for now to avoid deadlock/double borrow problem.
        drop(process_inner);
        recycle_res.clear();

        let mut process_inner = process.inner_mut();
        process_inner.children.clear();

        // deallocate other data in user space i.e. program code/data section
        let mut memory_set = process.memory_set.write();
        memory_set.recycle_data_pages();
        // drop file descriptors
        drop(memory_set);

        let mut fd_table = process.fd_table.write();
        fd_table.clear();
        drop(fd_table);
    }
    drop(process);
    // we do not have to save task context
    let mut _unused = TaskContext::empty();
    schedule(&mut _unused as *mut _);
}

pub fn hanging_current_and_run_next(sleep_time: usize, duration: usize) {
    let task = current_task().unwrap();
    let mut inner = task.inner_mut();
    let current_cx_ptr = &mut inner.task_cx as *mut TaskContext;
    inner.task_status = TaskStatus::Hanging;
    drop(inner);
    drop(task);
    acquire_processor().hang_current(sleep_time, duration);
    schedule(current_cx_ptr);
    panic!("Shouldn't reach here in `exit_current_and_run_next`!")
}

pub fn block_current_and_run_next() {
    let task = current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_mut();
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
    // add_task(INITPROC.clone());
    // 由于目前是在 fork 最后就 add_task，所以这里不需要再次 add_task;
    // 用于 lazy initproc
    let _initproc = INITPROC.clone();
}

pub fn exec_signal_handlers() {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_mut();
    if task_inner.pending_signals == SigSet::empty() {
        // {
        //     let process = task.process.upgrade().unwrap();
        //     let pid = process.pid();
        //     let tid = task_inner.tid();
        //     info!("exec_signal pid:{:?} tid:{:?}", pid, tid);
        // }
        return;
    }

    loop {
        // 取出 pending 的第一个 signal
        let signum = match task_inner
            .pending_signals
            .difference(task_inner.sigmask)
            .fetch()
        {
            Some(s) => s,
            None => return,
        };
        task_inner.pending_signals.sub(signum);

        let process = task.process.upgrade().unwrap();
        let sigaction = process.sigactions.read()[signum as usize];

        // 如果信号对应的处理函数存在，则做好跳转到 handler 的准备
        let handler = sigaction.sa_handler;
        match handler {
            SIG_IGN => {
                // return;
                continue; // loop
            }
            SIG_DFL => {
                // info!("default handler for signal {}", signum);
                if signum == Signal::SIGKILL as u32 || signum == Signal::SIGSEGV as u32 {
                    drop(task_inner);
                    drop(task);
                    exit_current_and_run_next(-(signum as i32), false);
                }
                return;
            }
            _ => {
                // info!("signal {} handler at {:x}", signum, handler);
                // 阻塞当前信号以及 sigaction.sa_mask 中的信号
                let mut sigmask = sigaction.sa_mask.clone();
                if !sigaction.sa_flags.contains(SAFlags::SA_NODEFER) {
                    sigmask.add(signum);
                }

                // 保存旧的信号掩码
                let old_sigmask = task_inner.sigmask.clone();
                sigmask.add_other(old_sigmask);
                // 将信号掩码设置为 sigmask
                task_inner.sigmask = sigmask;
                // 将 SignalContext 数据放入栈中
                let trap_cx = task_inner.trap_cx_mut();
                // 保存 Trap 上下文与 old_sigmask 到 sig_context 中
                let sig_context = SignalContext::from_another(trap_cx, old_sigmask);
                trap_cx.x[10] = signum as usize; // a0 (args0 = signum)
                                                 // 如果 sa_flags 中包含 SA_SIGINFO，则将 siginfo 和 ucontext 放入栈中

                let memory_set = process.memory_set.read();
                let token = memory_set.token();
                drop(memory_set);

                trap_cx.x[2] -= core::mem::size_of::<UContext>(); // sp -= sizeof(ucontext)
                let ucontext_ptr = trap_cx.x[2];
                trap_cx.x[2] -= core::mem::size_of::<SigInfo>(); // sp -= sizeof(siginfo)
                let siginfo_ptr = trap_cx.x[2];

                trap_cx.x[11] = siginfo_ptr; // a1 (args1 = siginfo)
                trap_cx.x[12] = ucontext_ptr; // a2 (args2 = ucontext)
                let mut ucontext = UContext::empty();
                ucontext.uc_mcontext.greps[1] = trap_cx.sepc; //pc
                copyout(token, ucontext_ptr as *mut UContext, &ucontext);

                trap_cx.x[2] -= core::mem::size_of::<SignalContext>(); // sp -= sizeof(sigcontext)
                let sig_context_ptr = trap_cx.x[2] as *mut SignalContext;
                copyout(token, sig_context_ptr, &sig_context);
                // *translated_mut(token, sig_context_ptr) = sig_context;

                trap_cx.x[1] = SIGNAL_TRAMPOLINE; // ra = user_sigreturn

                println!(
                    "prepare to jump to `handler`:{:x?}, original sepc = {:#x?},current sp:{:x?}",
                    handler, trap_cx.sepc, trap_cx.x[2]
                );
                trap_cx.sepc = handler; // sepc = handler
                drop(task_inner);
                return;
            }
        }
    }
}
