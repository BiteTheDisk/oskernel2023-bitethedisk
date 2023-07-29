use core::fmt::Debug;

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use nix::CloneFlags;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    add_task,
    manager::insert_into_pid2process,
    pid::{pid_alloc, PidHandle},
    signals::{SigAction, MAX_SIGNUM},
    task::{RecycleAllocator, TaskControlBlock, FD_LIMIT},
};
use crate::{
    consts::{PAGE_SIZE, USER_HEAP_SIZE},
    fs::{file::File, AbsolutePath, Stdin, Stdout},
    mm::{
        kernel_vmm::acquire_kvmm,
        memory_set::{AuxEntry, LoadedELF},
        translated_mut, MemorySet, MmapFlags, MmapProts, VirtAddr, VirtPageNum,
    },
    trap::{handler::user_trap_handler, TrapContext},
};

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    pub tgid: usize,
    pub pgid: usize,

    pub sigactions: Arc<RwLock<[SigAction; MAX_SIGNUM as usize]>>,
    pub memory_set: Arc<RwLock<MemorySet>>,
    pub fd_table: Arc<RwLock<FDTable>>,

    inner: Arc<RwLock<ProcessControlBlockInner>>,
}

pub struct ProcessControlBlockInner {
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: Option<i32>,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
    pub cwd: AbsolutePath,
}

type FDTable = Vec<Option<Arc<dyn File>>>;

impl ProcessControlBlockInner {
    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task_with_tid(&self, tid: usize) -> Option<&Arc<TaskControlBlock>> {
        self.tasks[tid].as_ref()
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }

    pub fn get_cwd(&self) -> AbsolutePath {
        self.cwd.clone()
    }

    pub fn is_zombie(&self) -> bool {
        self.exit_code.is_some()
    }
}

impl Debug for ProcessControlBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskControlBlock")
            .field("pid", &self.pid.0)
            .finish()
    }
}

impl ProcessControlBlock {
    /// 查找空闲文件描述符下标
    ///
    /// 从文件描述符表中 **由低到高** 查找空位，返回向量下标，没有空位则在最后插入一个空位
    pub fn alloc_fd(fd_table: &mut FDTable) -> usize {
        if let Some(fd) = (0..fd_table.len()).find(|fd| fd_table[*fd].is_none()) {
            fd
        } else {
            if fd_table.len() == FD_LIMIT {
                return FD_LIMIT;
            }
            fd_table.push(None);
            fd_table.len() - 1
        }
    }

    pub fn inner_ref(&self) -> RwLockReadGuard<'_, ProcessControlBlockInner> {
        self.inner.read()
    }

    pub fn inner_mut(&self) -> RwLockWriteGuard<'_, ProcessControlBlockInner> {
        self.inner.write()
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn new_from_elf(elf: Arc<dyn File>) -> Arc<Self> {
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        // memory_set with elf program headers/trampoline/trap context/user stack
        let LoadedELF {
            memory_set,
            elf_entry: entry_point,
            user_stack_top: user_sp,
            auxs,
        } = MemorySet::load_elf(elf.clone());
        // 为进程分配 PID 以及内核栈，并记录下内核栈在内核地址空间的位置
        // allocate a pid
        let pid_handle = pid_alloc();
        let tgid = pid_handle.0; // threads group id is eq to process id
        let pgid = pid_handle.0; // from elf, process group id is eq to process id

        let process = Arc::new(Self {
            pid: pid_handle,
            tgid,
            pgid,
            sigactions: Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize])),
            fd_table: Arc::new(RwLock::new(vec![
                // 0 -> stdin
                Some(Arc::new(Stdin)),
                // 1 -> stdout
                Some(Arc::new(Stdout)),
                // 2 -> stderr
                Some(Arc::new(Stdout)),
            ])),
            memory_set: Arc::new(RwLock::new(memory_set)),
            inner: Arc::new(RwLock::new(ProcessControlBlockInner {
                exit_code: None,
                parent: None,
                children: Vec::with_capacity(10),
                task_res_allocator: RecycleAllocator::new(),
                tasks: Vec::with_capacity(10),
                cwd: AbsolutePath::from_str("/"),
            })),
        });

        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(Arc::clone(&process), user_sp, true));

        // prepare trap_cx of main thread
        let task_inner = task.inner_mut();
        let trap_cx = task_inner.trap_cx_mut();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.top();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            acquire_kvmm().token(),
            kstack_top,
            user_trap_handler as usize,
        );

        // add main thread to the process
        let mut process_inner = process.inner_mut();
        process_inner.tasks.push(Some(Arc::clone(&task)));

        drop(task_inner);
        drop(process_inner);

        insert_into_pid2process(process.get_pid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    // https://refspecs.linuxbase.org/ELF/zSeries/lzsabi0_zSeries.html
    // http://refspecs.linux-foundation.org/elf/elfspec_ppc.pdf
    pub fn init_ustack(
        &self,
        user_sp: usize,
        args: Vec<String>,
        envs: Vec<String>,
        auxv: &mut Vec<AuxEntry>,
    ) -> (usize, usize, usize, usize) {
        let memory_set = self.memory_set.read();
        let token = memory_set.token();
        drop(memory_set);

        let mut user_sp = user_sp;

        // 先进行进行对齐 (qemu 暂时不用)
        // 计算共需要多少字节的空间
        // let mut total_len = 0;
        // for i in 0..envs.len() {
        //     total_len += envs[i].len() + 1; // String 不包含 '\0'
        // }
        // for i in 0..args.len() {
        //     total_len += args[i].len() + 1;
        // }
        // let align = core::mem::size_of::<usize>() / core::mem::size_of::<u8>(); // 8
        // let mut user_sp = user_sp - (align - total_len % align) * core::mem::size_of::<u8>();
        // user_sp -= core::mem::size_of::<usize>();

        // 分配 envs 的空间, 加入动态链接库位置
        let envs_ptrv: Vec<_> = (0..envs.len())
            .map(|idx| {
                user_sp -= envs[idx].len() + 1; // 1 是手动添加结束标记的空间('\0')
                let mut ptr = user_sp;
                for c in envs[idx].as_bytes() {
                    // 将参数写入到用户栈
                    *translated_mut(token, ptr as *mut u8) = *c;
                    ptr += 1;
                } // 写入字符串结束标记
                *translated_mut(token, ptr as *mut u8) = 0;
                user_sp
            })
            .collect();

        // 分配 args 的空间, 并写入字符串数据, 把字符串首地址保存在 argv 中
        // 这里高地址放前面的参数, 即先存放 argv[0]
        let args_ptrv: Vec<_> = (0..args.len())
            .map(|idx| {
                user_sp -= args[idx].len() + 1; // 1 是手动添加结束标记的空间('\0')
                let mut ptr = user_sp;
                for c in args[idx].as_bytes() {
                    // 将参数写入到用户栈
                    *translated_mut(token, ptr as *mut u8) = *c;
                    ptr += 1;
                } // 写入字符串结束标记
                *translated_mut(token, ptr as *mut u8) = 0;
                user_sp
            })
            .collect();

        // padding 0 表示结束 AT_NULL aux entry
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // 分配 auxs 空间，并写入数据
        user_sp -= auxv.len() * core::mem::size_of::<AuxEntry>();
        let auxs_base_ptr = user_sp;
        for i in 0..auxv.len() {
            user_sp -= core::mem::size_of::<AuxEntry>();
            *translated_mut(token, user_sp as *mut AuxEntry) = auxv[i];
        }
        // auxv.push(AuxEntry(AT_EXECFN,args_ptrv[0] ));

        // padding 0 表示结束
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // envs_ptr
        user_sp -= (envs.len()) * core::mem::size_of::<usize>();
        let envs_base_ptr = user_sp; // 参数字符串指针起始地址
        for i in 0..envs.len() {
            *translated_mut(
                token,
                (envs_base_ptr + i * core::mem::size_of::<usize>()) as *mut usize,
            ) = envs_ptrv[i];
        }

        // padding 0 表示结束
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = 0;

        // args_ptr
        user_sp -= (args.len()) * core::mem::size_of::<usize>();
        let args_base_ptr = user_sp; // 参数字符串指针起始地址
        for i in 0..args.len() {
            *translated_mut(
                token,
                (args_base_ptr + i * core::mem::size_of::<usize>()) as *mut usize,
            ) = args_ptrv[i];
        }

        // argc
        user_sp -= core::mem::size_of::<usize>();
        *translated_mut(token, user_sp as *mut usize) = args.len();

        (
            user_sp,
            args_base_ptr as usize,
            envs_base_ptr as usize,
            auxs_base_ptr as usize,
        )
    }

    /// 用来实现 exec 系统调用，即当前进程加载并执行另一个 ELF 格式可执行文件
    /// Only support processes with a single thread.
    pub fn exec(&self, elf_file: Arc<dyn File>, args: Vec<String>, envs: Vec<String>) {
        // 从 ELF 文件生成一个全新的地址空间并直接替换
        let LoadedELF {
            memory_set,
            user_stack_top: user_sp,
            elf_entry: entry_point,
            mut auxs,
        } = MemorySet::load_elf(elf_file);

        // memory_set
        // 这将导致原有的地址空间生命周期结束, 里面包含的全部物理页帧都会被回收,
        // 结果表现为: 原有的地址空间中的所有页表项的 ppn 引用计数减 1
        let mut ms = self.memory_set.write();
        *ms = memory_set;
        drop(ms); // 避免接下来的操作导致死锁

        // fd_table
        let mut fd_table = self.fd_table.write();
        fd_table
            .iter_mut()
            .find(|fd| fd.is_some() && !fd.as_ref().unwrap().available())
            .take(); // TODO

        // then we alloc user resource for main thread again
        // since memory_set has been changed
        let inner = self.inner_ref();
        let task = inner.get_task_with_tid(0).unwrap().clone();
        drop(inner);
        let mut task_inner = task.inner_mut();
        let res = task_inner.res.as_mut().unwrap();
        res.ustack_top = user_sp;
        res.alloc_user_res();
        let trap_cx_ppn = res.trap_cx_ppn();
        let user_sp = res.ustack_top();
        task_inner.trap_cx_ppn = trap_cx_ppn;
        let trap_cx = task_inner.trap_cx_mut();
        let (user_sp, args_ptr, envs_ptr, auxs_ptr) =
            self.init_ustack(user_sp, args, envs, &mut auxs);
        // 修改新的地址空间中的 Trap 上下文，将解析得到的应用入口点、用户栈位置以及一些内核的信息进行初始化
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            acquire_kvmm().token(),
            task.kstack.top(),
            user_trap_handler as usize,
        );
        trap_cx.x[10] = 0;
        trap_cx.x[11] = args_ptr;
        trap_cx.x[12] = envs_ptr;
        trap_cx.x[13] = auxs_ptr;
        drop(task_inner);
    }

    /// 用来实现 fork 系统调用，即当前进程 fork 出来一个与之几乎相同的子进程
    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>, flags: CloneFlags, stack: usize, newtls: usize) -> Arc<Self> {
        let mut parent = self.inner_mut();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        // 复制trap_cx和ustack等内存区域均在这里
        // 因此后面不需要再alloc_user_res了
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set.clone()
        } else {
            Arc::new(RwLock::new(MemorySet::from_copy_on_write(
                &mut self.memory_set.write(),
            )))
        };

        let mut fd_table = Vec::with_capacity(FD_LIMIT);
        // parent fd table
        let pfd_table_ref = self.fd_table.read();
        for fd in pfd_table_ref.iter() {
            if let Some(file) = fd {
                fd_table.push(Some(file.clone()));
            } else {
                fd_table.push(None);
            }
        }

        let pid_handle = pid_alloc();
        let tgid = if flags.contains(CloneFlags::THREAD) {
            self.pid.0
        } else {
            pid_handle.0
        };
        let pgid = if flags.contains(CloneFlags::THREAD) {
            self.pgid
        } else {
            pid_handle.0
        };

        let sigactions = if flags.contains(CloneFlags::SIGHAND) {
            self.sigactions.clone()
        } else {
            // parent sigactions
            let psa_ref = self.sigactions.read();
            let sa = Arc::new(RwLock::new([SigAction::new(); MAX_SIGNUM as usize]));
            let mut sa_mut = sa.write();
            for i in 1..MAX_SIGNUM as usize {
                sa_mut[i] = psa_ref[i].clone();
            }
            drop(sa_mut);
            sa
        };

        // create child process pcb
        let child = Arc::new(Self {
            pid: pid_handle,
            tgid,
            pgid,
            sigactions,
            memory_set,
            fd_table: Arc::new(RwLock::new(fd_table)),
            inner: Arc::new(RwLock::new(ProcessControlBlockInner {
                parent: Some(Arc::downgrade(self)),
                children: Vec::with_capacity(10),
                exit_code: None,
                tasks: Vec::with_capacity(10),
                cwd: parent.cwd.clone(),
                task_res_allocator: RecycleAllocator::new(),
            })),
        });

        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task_with_tid(0)
                .unwrap()
                // .clone()
                .inner_mut()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));

        // attach task to child process
        let mut child_inner = child.inner_mut();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);

        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_ref();
        let trap_cx = task_inner.trap_cx_mut();
        trap_cx.kernel_sp = task.kstack.top();

        // sys_fork return value
        if stack != 0 {
            trap_cx.set_sp(stack);
        }

        trap_cx.x[10] = 0;
        if flags.contains(CloneFlags::SETTLS) {
            trap_cx.x[4] = newtls;
        }

        drop(task_inner);
        insert_into_pid2process(child.get_pid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }

    /// 尝试用时加载缺页，目前只支持mmap缺页
    ///
    /// - 参数：
    ///     - `va`：缺页中的虚拟地址
    ///     - `is_load`：加载(1)/写入(0)
    /// - 返回值：
    ///     - `0`：成功加载缺页
    ///     - `-1`：加载缺页失败
    ///
    /// 分别用于：
    ///     - 用户态：handler page fault
    ///     - 内核态： translate_bytes_buffer
    pub fn check_lazy(&self, va: VirtAddr) -> isize {
        let mut memory_set = self.memory_set.write();

        let mmap_start = memory_set.mmap_manager.mmap_start;
        let mmap_end = memory_set.mmap_manager.mmap_top;
        let heap_start = VirtAddr::from(memory_set.brk_start);
        let heap_end = VirtAddr::from(memory_set.brk_start + USER_HEAP_SIZE);

        // fork
        let vpn: VirtPageNum = va.floor();
        let pte = memory_set.translate(vpn);
        if pte.is_some() && pte.unwrap().is_cow() {
            let former_ppn = pte.unwrap().ppn();
            return memory_set.cow_alloc(vpn, former_ppn);
        } else {
            if let Some(pte1) = pte {
                if pte1.is_valid() {
                    return -4;
                }
            }
        }

        // println!("check_lazy: va: {:#x}", va.0);

        // lazy map / lazy alloc heap
        if va >= heap_start && va <= heap_end {
            memory_set.lazy_alloc_heap(va.floor())
        } else if va >= mmap_start && va < mmap_end {
            memory_set.lazy_mmap(vpn);
            0
        } else {
            warn!("[check_lazy] {:x?}", va);
            warn!("[check_lazy] mmap_start: 0x{:x}", mmap_start.0);
            warn!("[check_lazy] mmap_end: 0x{:x}", mmap_end.0);
            warn!("[check_lazy] heap_start: 0x{:x}", heap_start.0);
            warn!("[check_lazy] heap_end: 0x{:x}", heap_end.0);
            -2
        }
    }

    // 在进程虚拟地址空间中分配创建一片虚拟内存地址映射
    pub fn mmap(
        &self,
        addr: usize,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        fd: isize,
        offset: usize,
    ) -> usize {
        if addr % PAGE_SIZE != 0 {
            panic!("mmap: addr not aligned");
        }

        let fd_table = self.fd_table.read().clone();
        // memory_set mut borrow
        let mut ms_mut = self.memory_set.write();
        let mut start_va = VirtAddr::from(0);
        // "prot<<1" 右移一位以符合 MapPermission 的权限定义
        // "1<<4" 增加 MapPermission::U 权限
        if addr == 0 {
            start_va = ms_mut.mmap_manager.get_mmap_top();
        }

        if flags.contains(MmapFlags::MAP_FIXED) {
            start_va = VirtAddr::from(addr);
            ms_mut.mmap_manager.remove(start_va, length);
        }
        let file = if flags.contains(MmapFlags::MAP_ANONYMOUS) {
            None
        } else {
            fd_table[fd as usize].clone()
        };
        ms_mut
            .mmap_manager
            .push(start_va, length, prot, flags, offset, file);
        drop(ms_mut);
        start_va.0
    }

    pub fn munmap(&self, addr: usize, length: usize) -> isize {
        let start_va = VirtAddr(addr);
        // 可能会有 mmap 后没有访问直接 munmap 的情况，需要检查是否访问过 mmap 的区域(即
        // 是否引发了 lazy_mmap)，防止 unmap 页表中不存在的页表项引发 panic
        self.memory_set
            .write()
            .mmap_manager
            .remove(start_va, length);
        0
    }

    pub fn pid(&self) -> usize {
        self.pid.0
    }

    pub fn grow_proc(&self, grow_size: isize) -> usize {
        // memory_set mut borrow
        let mut ms_mut = self.memory_set.write();
        let brk = ms_mut.brk;
        let brk_start = ms_mut.brk_start;
        if grow_size > 0 {
            let growed_addr: usize = brk + grow_size as usize;
            let limit = brk_start + USER_HEAP_SIZE;
            if growed_addr > limit {
                panic!(
                        "process doesn't have enough memsize to grow! limit:0x{:x}, heap_pt:0x{:x}, growed_addr:0x{:x}, pid:{}",
                        limit,
                        brk,
                        growed_addr,
                        self.pid.0
                    );
            }
            ms_mut.brk = growed_addr;
        } else {
            let shrinked_addr: usize = brk + grow_size as usize;
            if shrinked_addr < brk_start {
                panic!("Memory shrinked to the lowest boundary!")
            }
            ms_mut.brk = shrinked_addr;
        }
        return ms_mut.brk;
    }
}
