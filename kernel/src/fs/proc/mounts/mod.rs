use crate::fs::fino_alloc;
use crate::fs::FInoHandle;
use crate::fs::File;
use crate::mm::UserBuffer;
use alloc::string::String;
use alloc::string::ToString;

#[derive(Debug)]
pub struct Mounts {
    fid: FInoHandle,
}

impl Mounts {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
    }
}

impl File for Mounts {
    fn name(&self) -> String {
        "mounts".to_string()
    }

    fn readable(&self) -> bool {
        true
    }

    fn file_size(&self) -> usize {
        usize::MAX
    }

    fn read_to_buf(&self, buf: UserBuffer) -> usize {
        buf.len()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
