mod meminfo;
mod mounts;

use alloc::vec::Vec;
pub use meminfo::*;
pub use mounts::*;
use path::AbsolutePath;
use spin::Mutex;

use crate::fs::{fsid_alloc, File, OpenFlags, VFS};

use crate::fs::FSIDHandle;
use crate::syscall::impls::Errno;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;

use crate::fs::{CreateMode, Dirent, FileType};

use super::KFile;

pub struct ProcFS {
    pub id: FSIDHandle,
    pub mount_path: AbsolutePath,
}

#[derive(Debug)]
pub struct ProcDir {
    fid: FSIDHandle,
    index: Mutex<usize>,
    dirents: Vec<Dirent>,
}

impl ProcDir {
    pub fn new() -> Self {
        Self {
            fid: fsid_alloc(),
            index: Mutex::new(0),
            dirents: vec![
                Dirent::new("meminfo", 0, 0, FileType::RegularFile),
                Dirent::new("mounts", 0, 0, FileType::RegularFile),
            ],
        }
    }
    pub fn index(&self) -> usize {
        *self.index.lock()
    }

    pub fn step(&self) {
        *self.index.lock() += 1;
    }
}

impl File for ProcDir {
    fn open(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(ProcDir::new())),
            "meminfo" => Ok(Arc::new(MemInfo::new(
                Arc::new(MemInfoInner::new()),
                path,
                flags,
            ))),
            "mounts" => Ok(Arc::new(Mounts::new(
                Arc::new(MountsInner::new()),
                path,
                flags,
            ))),
            _ => Err(Errno::ENOENT),
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
        "proc".to_string()
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

    fn ino(&self) -> usize {
        self.fid.get()
    }
}

impl ProcFS {
    pub fn init(mount_path: AbsolutePath) -> Self {
        let id = fsid_alloc();
        Self { id, mount_path }
    }
}

impl VFS for ProcFS {
    fn mount_path(&self) -> AbsolutePath {
        self.mount_path.clone()
    }

    fn root_dir(&self, _mode: OpenFlags) -> Arc<dyn File> {
        Arc::new(ProcDir::new())
    }

    fn name(&self) -> String {
        "procfs".to_string()
    }
}
