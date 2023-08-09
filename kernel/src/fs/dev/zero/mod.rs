use crate::fs::{fino_alloc, FInoHandle, File, Kstat, S_IFCHR, S_IFREG};

use alloc::string::ToString;
use alloc::{string::String, vec::Vec};

// 成员待完善
#[derive(Debug)]
pub struct Zero {
    fid: FInoHandle,
}

impl Zero {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
    }
}
impl File for Zero {
    fn read(&self, _offset: usize, len: usize) -> Vec<u8> {
        let mut zero = Vec::with_capacity(len);
        for _ in 0..len {
            zero.push(0);
        }
        zero
    }

    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        false
    }

    fn name(&self) -> String {
        "zero".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }

    fn fstat(&self, kstat: &mut Kstat) {
        kstat.st_mode = S_IFCHR;
    }
}
