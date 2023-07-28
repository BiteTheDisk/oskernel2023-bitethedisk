pub const USER_STACK_SIZE: usize = 4096 * 2048;
pub const KERNEL_STACK_SIZE: usize = 4096 * 32; // 应用进程在内核的栈大小

pub const USER_HEAP_SIZE: usize = 4096 * 48;
pub const KERNEL_HEAP_SIZE: usize = 4096 * 8192; // 32M

pub const PHYS_END: usize = 0x88000000; // 128 MiB

/// 页面大小：4KiB
pub const PAGE_SIZE: usize = 0x1000;

/// 跳板虚拟内存中的起始地址，虚拟内存最高页
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// 用于存放信号处理函数的栈
pub const SIGNAL_TRAMPOLINE: usize = TRAMPOLINE - PAGE_SIZE;

/// Trap 上下文在应用地址空间中的位置
pub const TRAP_CONTEXT_BASE: usize = SIGNAL_TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MMIO};

/// 进程用户栈基址
pub const USER_STACK_BASE: usize = 0xf000_0000;
