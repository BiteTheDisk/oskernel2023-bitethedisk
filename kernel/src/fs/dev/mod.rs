pub mod misc;
pub mod null;
pub mod sda2;
mod shm;
pub mod tty;
pub mod zero;

pub use misc::*;
pub use null::*;
pub use sda2::*;
pub use shm::*;
pub use tty::*;
pub use zero::*;

use alloc::vec::Vec;
use spin::Mutex;

use path::AbsolutePath;

use crate::fs::FSIDHandle;
use crate::syscall::impls::Errno;

use super::Dirent;
use super::KFile;
use crate::fs::fsid_alloc;

use crate::fs::VFS;

use crate::fs::CreateMode;
use crate::fs::File;

use crate::fs::FileType;
use crate::fs::Kstat;
use crate::fs::OpenFlags;
use crate::fs::S_IFDIR;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;

// TODO 成员待完善
#[derive(Debug)]
pub struct DevDir {
    index: Mutex<usize>,
    dirents: Vec<Dirent>,

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
            dirents: vec![
                Dirent::new("sda2", 0, 0, FileType::BlockDevice),
                Dirent::new("pts", 0, 0, FileType::CharDevice),
                Dirent::new("null", 0, 0, FileType::CharDevice),
                Dirent::new("zero", 0, 0, FileType::CharDevice),
                Dirent::new("misc", 0, 0, FileType::Directory),
                Dirent::new("shm", 0, 0, FileType::Directory),
            ],

            fid: fsid_alloc(),
        }
    }

    pub fn index(&self) -> usize {
        *self.index.lock()
    }

    pub fn step(&self) {
        *self.index.lock() += 1;
    }
}

impl File for DevDir {
    // TODO 待完善
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
        // vec.push(("sda2".to_string(), S_IFBLK));
        // vec.push(("pts".to_string(), S_IFCHR));
        // vec.push(("null".to_string(), S_IFCHR));
        // vec.push(("zero".to_string(), S_IFCHR));
        // vec.push(("misc".to_string(), S_IFDIR));

        for dirent in self.dirents.iter() {
            vec.push(dirent.info());
        }
        vec
    }

    fn is_dir(&self) -> bool {
        true
    }

    // fn all_dirents(&self) -> Option<Vec<Dirent>> {
    //     let mut dirents = Vec::new();

    //     // dirents.push(Dirent::new("pts", 0, 0, FileType::CharDevice));
    //     // dirents.push(Dirent::new("null", 0, 0, FileType::CharDevice));
    //     // dirents.push(Dirent::new("zero", 0, 0, FileType::CharDevice));
    //     // dirents.push(Dirent::new("misc", 0, 0, FileType::Directory));

    //     dirents = self.dirents.clone();

    //     Some(dirents)
    // }

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

    fn open(
        &self,
        path: AbsolutePath,
        open_flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match path.name().as_str() {
            "/" => Ok(Arc::new(DevDir::new())),
            "vda2" | "sda2" => Ok(Arc::new(SDA2::new())),
            "tty" | "pts" => Ok(Arc::new(TTY::new())),
            "null" => Ok(Arc::new(Null::new())),
            "zero" => Ok(Arc::new(Zero::new())),
            "shm" => Ok(Arc::new(ShmDir::new())),
            "misc" => Ok(Arc::new(MiscDir::new())),
            _ => Ok(Arc::new(DevFile::new(
                Arc::new(_DevFile::new()),
                path,
                open_flags,
            ))),
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

#[derive(Debug, Default, Clone, Copy)]
pub struct _DevFile;

impl _DevFile {
    pub fn new() -> Self {
        Self {}
    }
}

pub type DevFile = KFile<_DevFile>;

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
