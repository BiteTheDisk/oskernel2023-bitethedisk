use crate::fs::fino_alloc;
use crate::fs::FInoHandle;
use crate::fs::File;
use crate::fs::{CharFile, DeviceFile};
use alloc::string::String;
use alloc::string::ToString;

#[derive(Debug)]
pub struct SDA2 {
    fid: FInoHandle,
}

impl SDA2 {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
    }
}

impl DeviceFile for SDA2 {}

impl CharFile for SDA2 {}

impl File for SDA2 {
    fn name(&self) -> String {
        "sda2".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
