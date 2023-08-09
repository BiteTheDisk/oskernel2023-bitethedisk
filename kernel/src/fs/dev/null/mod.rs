use crate::fs::{CharFile, DeviceFile};
use crate::fs::{File, KFile, Kstat, S_IFCHR};
use alloc::string::ToString;
use alloc::{string::String, vec::Vec};

#[derive(Debug)]
pub struct NullInner {}

pub type Null = KFile<NullInner>;

impl NullInner {
    pub fn new() -> Self {
        Self {}
    }
}

impl File for Null {
    fn writable(&self) -> bool {
        true
    }

    fn write(&self, data: &Vec<u8>) -> usize {
        data.len()
    }

    fn name(&self) -> String {
        "null".to_string()
    }

    fn ino(&self) -> usize {
        self.inner.lock().fid.get()
    }

    fn fstat(&self, kstat: &mut Kstat) {
        kstat.st_mode = S_IFCHR;
    }
}

impl DeviceFile for Null {}
impl CharFile for Null {}
