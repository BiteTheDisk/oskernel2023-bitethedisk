use crate::fs::{File, KFile, Kstat, S_IFCHR};

use alloc::string::ToString;
use alloc::{string::String, vec::Vec};

// 成员待完善
#[derive(Debug)]
pub struct ZeroInner {}

pub type Zero = KFile<ZeroInner>;

impl ZeroInner {
    pub fn new() -> Self {
        Self {}
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
        self.inner.lock().fid.get()
    }

    fn fstat(&self, kstat: &mut Kstat) {
        kstat.st_mode = S_IFCHR;
    }
}
