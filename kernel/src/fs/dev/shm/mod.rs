use crate::fs::{fino_alloc, CreateMode, Dirent, FInoHandle, File, OpenFlags};
use crate::syscall::impls::Errno;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::{string::String, vec::Vec};
use path::AbsolutePath;
use spin::Mutex;

#[derive(Debug)]
pub struct ShmDir {
    index: Mutex<usize>,
    dirents: Vec<Dirent>,

    fid: FInoHandle,
}

impl ShmDir {
    pub fn new() -> Self {
        Self {
            index: Mutex::new(0),
            dirents: Vec::new(),
            fid: fino_alloc(),
        }
    }

    pub fn index(&self) -> usize {
        *self.index.lock()
    }

    pub fn step(&self) {
        *self.index.lock() += 1;
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
        _flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(ShmDir::new())),
            _ => Err(Errno::ENOENT),
        }
    }

    fn is_dir(&self) -> bool {
        true
    }

    fn dirent(&self, dirent: &mut Dirent) -> isize {
        let index = self.index();
        if index < self.dirents.len() {
            *dirent = self.dirents[index].clone();
            self.step();
            core::mem::size_of::<Dirent>() as isize
        } else {
            -1
        }
    }

    fn ls_with_attr(&self) -> Vec<(String, u32)> {
        let mut vec = Vec::new();
        for dirent in self.dirents.iter() {
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
