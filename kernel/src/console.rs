//! 控制台输出输出

use core::{
    fmt::{self, Write},
    ops::Deref,
};

use alloc::{collections::BTreeMap, string::String};
use spin::Mutex;

use crate::{sbi::console_putchar, task::current_task};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as i32);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    Stdout.write_fmt(args).unwrap();
}

pub fn buffer_print(buf: &str) {
    let pid = current_task().unwrap().pid();
    let mut console_buffer_manager = CONSOLE_BUFFER_MANAGER.lock();
    if let Some(console_buffer) = console_buffer_manager.peek_output_buffer(pid) {
        console_buffer.push_str(buf);
    } else {
        let console_buffer = console_buffer_manager.fetch_new_buffer(pid);
        console_buffer.push_str(buf);
    }
}
pub fn print_buffer(pid: Pid) {
    let mut console_buffer_manager = CONSOLE_BUFFER_MANAGER.lock();
    if let Some(buf) = console_buffer_manager.get_output_buffer(pid) {
        Stdout.write_str(&buf).unwrap();
    }
}

#[macro_export]
macro_rules! print {
    ($fmt:literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::console::print("\n");
    };
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    };
    ($($arg:tt)+) => {
        $crate::console::print(format_args!($($arg)+));
    };
}

type Pid = usize;
pub struct ConsoleBuffer {
    inner: String,
}

impl core::ops::DerefMut for ConsoleBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for ConsoleBuffer {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl ConsoleBuffer {
    fn new() -> ConsoleBuffer {
        ConsoleBuffer {
            inner: String::new(),
        }
    }
}

pub struct ConsoleBufferManager {
    inner: BTreeMap<Pid, ConsoleBuffer>,
}
impl ConsoleBufferManager {
    pub const fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }
    pub fn get_output_buffer(&mut self, pid: Pid) -> Option<ConsoleBuffer> {
        self.inner.remove(&pid)
    }
    pub fn peek_output_buffer(&mut self, pid: Pid) -> Option<&mut ConsoleBuffer> {
        self.inner.get_mut(&pid)
    }
    pub fn fetch_new_buffer(&mut self, pid: Pid) -> &mut ConsoleBuffer {
        self.inner.insert(pid, ConsoleBuffer::new());
        self.inner.get_mut(&pid).unwrap()
    }
}

pub static CONSOLE_BUFFER_MANAGER: Mutex<ConsoleBufferManager> =
    Mutex::new(ConsoleBufferManager::new());
