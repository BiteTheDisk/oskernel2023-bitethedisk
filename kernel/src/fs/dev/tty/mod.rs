use alloc::string::String;
use alloc::string::ToString;

use crate::fs::fino_alloc;
use crate::fs::FInoHandle;
use crate::fs::File;
#[derive(Debug)]
pub struct TTY {
    fid: FInoHandle,
}

impl TTY {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
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
        self.fid.get()
    }
}

use crate::fs::{CharFile, DeviceFile};

impl DeviceFile for TTY {}
impl CharFile for TTY {}
