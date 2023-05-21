//! trap 上下文

use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器`x0 ~ x31`
    pub x: [usize; 32],

    /// 提供状态信息
    pub sstatus: Sstatus,

    /// 记录 Trap 发生之前执行的最后一条指令的地址
    pub sepc: usize,

    // 一下数据在应用初始化的时候由内核写入应用地址空间中的TrapContext中的相应位置，此后不再修改
    /// 内核地址空间的 token ，即内核页表的起始物理地址
    pub kernel_satp: usize,

    /// 当前应用在内核地址空间中的内核栈栈顶的虚拟地址
    pub kernel_sp: usize,

    /// 内核中 trap handler 入口点的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let sstatus = sstatus::read();
        unsafe { sstatus::set_spp(SPP::User) }
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // Trap 返回后到程序入口
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
