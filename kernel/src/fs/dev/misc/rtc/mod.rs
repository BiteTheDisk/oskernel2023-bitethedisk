use crate::fs::fino_alloc;
use crate::fs::DeviceFile;
use crate::fs::FInoHandle;

use alloc::string::String;
use alloc::string::ToString;

use crate::fs::File;
#[derive(Debug)]
pub struct Rtc {
    pub fid: FInoHandle,
}

impl Rtc {
    pub fn new() -> Self {
        Self { fid: fino_alloc() }
    }
}

impl DeviceFile for Rtc {}

impl File for Rtc {
    fn name(&self) -> String {
        "rtc".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
