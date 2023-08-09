use crate::fs::File;
use crate::fs::KFile;
use alloc::string::String;
use alloc::string::ToString;
#[derive(Debug)]
pub struct MemInfoInner {}
pub type MemInfo = KFile<MemInfoInner>;

impl MemInfoInner {
    pub fn new() -> Self {
        Self {}
    }
}

impl File for MemInfo {
    fn name(&self) -> String {
        "meminfo".to_string()
    }

    fn ino(&self) -> usize {
        self.inner.lock().fid.get()
    }
}
