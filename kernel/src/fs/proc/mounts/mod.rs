use crate::fs::File;
use crate::fs::KFile;
use crate::mm::UserBuffer;
use alloc::string::String;
use alloc::string::ToString;

#[derive(Debug, Default, Clone, Copy)]
pub struct MountsInner;
pub type Mounts = KFile<MountsInner>;

impl MountsInner {
    pub fn new() -> Self {
        Self {}
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
        self.inner.lock().fid.get()
    }

    // TODO lzm
    fn offset(&self) -> usize {
        self.inner.lock().offset
    }
}
