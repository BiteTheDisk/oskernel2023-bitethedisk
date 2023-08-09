use crate::fs::{dirent, fino_alloc, CreateMode, Dirent, FInoHandle, File, OpenFlags};
use crate::syscall::impls::Errno;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::{string::String, vec::Vec};
use dirent::FileType;
use path::AbsolutePath;
use spin::Mutex;

use super::{DevFile, DevFileInner};

#[derive(Debug)]
pub struct ShmDir {
    index: Mutex<usize>,
    dirents: Mutex<Vec<Dirent>>,
    fid: FInoHandle,
}

impl ShmDir {
    pub fn new() -> Self {
        Self {
            index: Mutex::new(0),
            dirents: Mutex::new(vec![]),
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

impl Default for ShmDir {
    fn default() -> Self {
        Self::new()
    }
}

impl File for ShmDir {
    fn open(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(ShmDir::new())),
            _ => {
                let dirents = Dirent::new(path.name().as_str(), 0, 0, FileType::UNKNOWN);
                self.add_dirent(dirents);
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
        "shm".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}
