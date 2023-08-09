pub use spin::Mutex;

use alloc::vec::Vec;

use crate::fs::{fino_alloc, Dirent, FInoHandle, FileType};

use crate::fs::{CreateMode, File, OpenFlags};
use crate::syscall::impls::Errno;

use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;

use path::AbsolutePath;

mod rtc;

pub use rtc::*;

use super::{DevFile, DevFileInner};

#[derive(Debug)]
pub struct MiscDir {
    index: Mutex<usize>,
    dirents: Mutex<Vec<Dirent>>,
    fid: FInoHandle,
}

impl MiscDir {
    pub fn new() -> Self {
        Self {
            index: Mutex::new(0),
            dirents: Mutex::new(vec![Dirent::new("rtc", 0, 0, FileType::BlockDevice)]),
            fid: fino_alloc(),
        }
    }

    pub fn index(&self) -> usize {
        *self.index.lock()
    }

    pub fn step(&self) {
        *self.index.lock() += 1;
    }

    pub fn add_dirent(&self, dirent: Dirent) {
        self.dirents.lock().push(dirent);
    }

    pub fn remove_dirent(&self, name: &str) {
        let mut dirents = self.dirents.lock();
        for (i, dirent) in dirents.iter().enumerate() {
            if dirent.name() == name {
                dirents.remove(i);
                return;
            }
        }
    }
}

impl Default for MiscDir {
    fn default() -> Self {
        Self::new()
    }
}

impl File for MiscDir {
    fn open(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(MiscDir::new())),
            "rtc" => Ok(Arc::new(Rtc::new(Arc::new(RtcInner::new()), path, flags))),
            _ => {
                let dirent = Dirent::new(path.name().as_str(), 0, 0, FileType::UNKNOWN);
                self.add_dirent(dirent);
                Ok(Arc::new(DevFile::new(
                    Arc::new(DevFileInner::new()),
                    path,
                    flags,
                )))
            }
        }
    }

    fn is_dir(&self) -> bool {
        true
    }

    fn dirent(&self, dirent: &mut Dirent) -> isize {
        let index = self.index();
        let dirents = self.dirents.lock();
        if index < dirents.len() {
            *dirent = dirents[index].clone();
            self.step();
            core::mem::size_of::<Dirent>() as isize
        } else {
            -1
        }
    }

    fn ls_with_attr(&self) -> Vec<(String, u32)> {
        let mut vec = Vec::new();
        let dirents = self.dirents.lock();
        for dirent in dirents.iter() {
            vec.push(dirent.info());
        }
        vec
    }

    fn name(&self) -> String {
        "misc".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
