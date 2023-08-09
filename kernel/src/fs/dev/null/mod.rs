use crate::fs::{fino_alloc, FInoHandle, File, Kstat, S_IFCHR, S_IFREG};
use crate::fs::{CharFile, DeviceFile};
use alloc::string::ToString;
use alloc::{string::String, vec::Vec};

#[derive(Debug)]
pub struct Null {
    fid: FInoHandle,
}

impl Null {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
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
        self.fid.get()
    }

    fn fstat(&self, kstat: &mut Kstat) {
        kstat.st_mode = S_IFCHR;
    }
}

impl DeviceFile for Null {}
impl CharFile for Null {}
