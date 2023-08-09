use crate::fs::fino_alloc;
use crate::fs::FInoHandle;
use crate::fs::File;
use alloc::string::String;
use alloc::string::ToString;
#[derive(Debug)]
pub struct MemInfo {
    fid: FInoHandle,
}

impl MemInfo {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
    }
}

impl File for MemInfo {
    fn name(&self) -> String {
        "meminfo".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
