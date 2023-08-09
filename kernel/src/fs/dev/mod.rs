mod misc;
mod null;
mod sda2;
mod shm;
mod tty;
mod zero;

use alloc::rc::Weak;
pub use misc::*;
pub use null::*;
pub use sda2::*;
pub use shm::*;
pub use tty::*;
pub use zero::*;

use alloc::vec::Vec;
use path::AbsolutePath;
use spin::Mutex;

use super::Dirent;
use super::KFile;
use crate::fs::fsid_alloc;
use crate::fs::CreateMode;
use crate::fs::FSIDHandle;
use crate::fs::File;
use crate::fs::FileType;
use crate::fs::Kstat;
use crate::fs::OpenFlags;
use crate::fs::S_IFDIR;
use crate::fs::VFS;
use crate::syscall::impls::Errno;

use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;

// TODO 成员待完善
#[derive(Debug)]
pub struct DevDir {
    index: Mutex<usize>,
    dirents: Mutex<Vec<Dirent>>,
    fid: FSIDHandle,
}

pub struct DevFS {
    pub id: FSIDHandle,
    pub mount_path: AbsolutePath,
}

impl DevDir {
    pub fn new() -> Self {
        Self {
            index: Mutex::new(0),
            dirents: Mutex::new(vec![
                Dirent::new("sda2", 0, 0, FileType::BlockDevice),
                Dirent::new("pts", 0, 0, FileType::CharDevice),
                Dirent::new("null", 0, 0, FileType::CharDevice),
                Dirent::new("zero", 0, 0, FileType::CharDevice),
                Dirent::new("misc", 0, 0, FileType::Directory),
                Dirent::new("shm", 0, 0, FileType::Directory),
            ]),

            fid: fsid_alloc(),
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

    pub fn remove_dirent(&self, name: &str) -> Result<(), Errno> {
        let mut index = 0;
        let mut lock = self.dirents.lock();
        for dirent in lock.iter() {
            if dirent.name() == name.to_string() {
                lock.remove(index);
                return Ok(());
            }
            index += 1;
        }
        Err(Errno::ENOENT)
    }
}

impl File for DevDir {
    fn fstat(&self, kstat: &mut Kstat) {
        let st_size = 0;
        let st_blksize = 0;
        let st_blocks = 0;
        let st_mode = S_IFDIR;

        kstat.init(
            st_size,
            st_blksize,
            st_blocks,
            st_mode,
            self.ino() as u64,
            0,
            0,
            0,
        );
    }

    fn ls_with_attr(&self) -> Vec<(String, u32)> {
        let mut vec = Vec::new();
        let dirents = self.dirents.lock();
        for dirent in dirents.iter() {
            vec.push(dirent.info());
        }
        vec
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

    fn open(
        &self,
        path: AbsolutePath,
        open_flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(DevDir::new())),
            "vda2" | "sda2" => Ok(Arc::new(SDA2::new(
                Arc::new(SDA2Inner::new()),
                path,
                open_flags,
            ))),
            "tty" | "pts" => Ok(Arc::new(TTY::new(
                Arc::new(TTYInner::new()),
                path,
                open_flags,
            ))),
            "null" => Ok(Arc::new(Null::new(
                Arc::new(NullInner::new()),
                path,
                open_flags,
            ))),
            "zero" => Ok(Arc::new(Zero::new(
                Arc::new(ZeroInner::new()),
                path,
                open_flags,
            ))),
            "shm" => Ok(Arc::new(ShmDir::new())),
            "misc" => Ok(Arc::new(MiscDir::new())),
            _ => {
                let dirent = Dirent::new(path.name().as_str(), 0, 0, FileType::UNKNOWN);
                self.add_dirent(dirent);
                Ok(Arc::new(DevFile::new(
                    Arc::new(DevFileInner::new()),
                    path,
                    open_flags,
                )))
            }
        }
    }

    fn name(&self) -> String {
        "dev".to_string()
    }

    fn ino(&self) -> usize {
        self.fid.get()
    }
}

impl DevFS {
    pub fn init(mount_path: AbsolutePath) -> Self {
        let id = fsid_alloc();
        Self { id, mount_path }
    }
}

impl VFS for DevFS {
    fn mount_path(&self) -> AbsolutePath {
        self.mount_path.clone()
    }

    fn root_dir(&self, _mode: OpenFlags) -> Arc<dyn File> {
        Arc::new(DevDir::new())
    }

    fn fsid(&self) -> usize {
        self.id.0
    }

    fn name(&self) -> String {
        "devfs".to_string()
    }
}

#[derive(Debug)]
pub struct DevFileInner {}

impl DevFileInner {
    pub fn new() -> Self {
        Self {}
    }
}

pub type DevFile = KFile<DevFileInner>;

impl File for DevFile {
    fn name(&self) -> String {
        self.path.name()
    }

    fn set_cloexec(&self) {
        self.inner.lock().flags |= OpenFlags::O_CLOEXEC;
    }

    // TODO lzm
    fn delete(&self) {}
}
