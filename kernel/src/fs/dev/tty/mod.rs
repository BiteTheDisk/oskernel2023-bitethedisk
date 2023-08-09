use alloc::string::String;
use alloc::string::ToString;

use crate::fs::File;
use crate::fs::KFile;
#[derive(Debug)]
pub struct TTYInner {}

pub type TTY = KFile<TTYInner>;

impl TTYInner {
    pub fn new() -> Self {
        Self {}
    }
}

impl File for TTY {
    fn name(&self) -> String {
        "tty".to_string()
    }

    fn available(&self) -> bool {
        true
    }

    fn set_cloexec(&self) {}

    fn ino(&self) -> usize {
        self.inner.lock().fid.get()
    }
}

use crate::fs::{CharFile, DeviceFile};

impl DeviceFile for TTY {}
impl CharFile for TTY {}
