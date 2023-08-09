use crate::fs::File;
use crate::fs::KFile;
use crate::fs::{CharFile, DeviceFile};
use alloc::string::String;
use alloc::string::ToString;

#[derive(Debug)]
pub struct SDA2Inner {}

pub type SDA2 = KFile<SDA2Inner>;

impl SDA2Inner {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceFile for SDA2 {}

impl CharFile for SDA2 {}

impl File for SDA2 {
    fn name(&self) -> String {
        "sda2".to_string()
    }

    fn ino(&self) -> usize {
        self.inner.lock().fid.get()
    }
}
