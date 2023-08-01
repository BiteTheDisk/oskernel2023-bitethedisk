#![no_std]
#![no_main]
// features
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![feature(error_in_core)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[macro_use]
mod macros;
#[macro_use]
pub mod console; // 控制台模块

#[path = "boards/qemu.rs"]
mod board; // 与虚拟机相关的参数

mod consts;
mod drivers; // 设备驱动层
mod error;
mod fs;
mod logging;
mod mm;
mod panic;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use crate::task::initproc;
use core::{
    arch::global_asm,
    sync::atomic::{AtomicBool, Ordering},
};
use riscv::register::sstatus::{set_fs, FS};

global_asm!(include_str!("entry.S"));

static MEOWED: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub fn meow() -> ! {
    if MEOWED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        println!("boot hart id: {}", hartid!());
        unsafe { set_fs(FS::Dirty) }
        // logging::init();
        mm::init();
        trap::init();
        trap::enable_stimer_interrupt();
        timer::set_next_trigger();
        fs::init();
        task::add_initproc();
        initproc::preload();
        initproc::setup_test_all2();
        toggle_booted!();
        task::run_tasks();
    } else {
        synchronize_hart!();
        mm::enable_mmu();
        trap::init();
        trap::enable_stimer_interrupt();
        timer::set_next_trigger();
        task::run_tasks();
    }

    unreachable!("main.rs/meow: you should not be here!");
}
