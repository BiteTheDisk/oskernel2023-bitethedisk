use crate::fs::DeviceFile;
use crate::fs::KFile;

use alloc::string::String;
use alloc::string::ToString;

use crate::fs::File;
#[derive(Debug)]
pub struct RtcInner;

pub type Rtc = KFile<RtcInner>;

impl RtcInner {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceFile for Rtc {}

impl File for Rtc {
    fn name(&self) -> String {
        "rtc".to_string()
    }

    fn ino(&self) -> usize {
        self.inner.lock().fid.get()
    }
}
