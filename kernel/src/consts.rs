pub const USER_STACK_SIZE: usize = 4096 * 2048;
/// Stack size of an application process in the kernel.
pub const KERNEL_STACK_SIZE: usize = 4096 * 32;

pub const USER_HEAP_SIZE: usize = 4096 * 30000;
pub const KERNEL_HEAP_SIZE: usize = 4096 * 8192; // 32M

pub const PHYS_END: usize = 0x8800_0000; // 128 MiB

pub const PAGE_SIZE: usize = 0x1000;

/// Starting address of the trampoline virtual memory,
/// the highest page in the virtual memory.
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// Stack used for storing signal handlers.
pub const SIGNAL_TRAMPOLINE: usize = TRAMPOLINE - PAGE_SIZE;

/// Location of the trap context in the application address space.
pub const TRAP_CONTEXT_BASE: usize = SIGNAL_TRAMPOLINE - PAGE_SIZE;

pub const USER_STACK_BASE: usize = 0xf000_0000;

pub const THREAD_LIMIT: usize = 4096 * 2;

pub use crate::board::{CLOCK_FREQ, MMIO};

pub const MMAP_BASE: usize = 0x6000_0000;

// pub const MMAP_END: usize = 0x68000000; // mmap 区大小为 128 MiB

pub const SHM_BASE: usize = 0x7000_0000;

pub const LINK_BASE: usize = 0x2000_0000;

pub const FD_LIMIT: usize = 1024;
