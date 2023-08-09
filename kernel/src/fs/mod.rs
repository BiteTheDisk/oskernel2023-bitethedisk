//! 内核 fs

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use spin::RwLock;

mod dev;
mod dirent;
mod fat32;
mod fdset;
mod mount;
mod open_flags;
mod pipe;
mod proc;
mod stat;
mod stdio;
mod traits;

use path::AbsolutePath;

pub use self::fat32::*;
pub use dirent::*;
pub use fdset::*;
pub use mount::*;
pub use open_flags::*;
pub use pipe::*;
pub use stat::*;
pub use stdio::*;
pub use traits::*;

pub fn init() {
    // TODO 挂载 proc 与 dev
    mount(
        "unknown".into(),
        "/proc".into(),
        "procfs".into(),
        (OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE).bits(),
    );

    mount(
        "unknown".into(),
        "/dev".into(),
        "devfs".into(),
        (OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE).bits(),
    );

    open(
        "/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var/tmp/lmbench".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();

    // TODO ?? lzm
    open("/lat_sig".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();

    println!("===+ Files Loaded +===");
    list_apps(AbsolutePath::from_str("/"));
    println!("===+==============+===");
}

#[repr(u64)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FSType {
    UNKNOWN,
    FAT32,
    DEV,
    PROC,
}

impl Default for FSType {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

/// 常规文件或目录
pub struct KFile<T> {
    pub(crate) inner: Mutex<KFileInner<T>>,
    pub(crate) path: AbsolutePath,
}

pub struct KFileInner<T> {
    pub(crate) fid: FInoHandle,
    pub(crate) offset: usize,
    pub(crate) inode: Arc<T>,
    pub(crate) available: bool,
    pub(crate) flags: OpenFlags,

    pub(crate) time_info: TimeInfo,
}

pub struct KFS<T> {
    pub(crate) fs: Arc<RwLock<T>>,
    pub(crate) id: FSIDHandle,
    pub(crate) mount_path: AbsolutePath,
}

pub struct IDAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl IDAllocator {
    pub const fn new() -> Self {
        IDAllocator {
            current: 0,
            recycled: vec![],
        }
    }
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == id),
            "id {} has been deallocated!",
            id
        );
        self.recycled.push(id);
    }
}

pub static FSID_ALLOCATOR: Mutex<IDAllocator> = Mutex::new(IDAllocator::new());
pub static FINO_ALLOCATOR: Mutex<IDAllocator> = Mutex::new(IDAllocator::new());

#[derive(Debug)]
pub struct FSIDHandle(usize);

// for kstat.st_ino
#[derive(Debug)]
pub struct FInoHandle(usize);

impl FSIDHandle {
    pub fn get(&self) -> usize {
        self.0
    }
}

impl FInoHandle {
    pub fn get(&self) -> usize {
        self.0
    }
}

impl Drop for FSIDHandle {
    fn drop(&mut self) {
        FSID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl Drop for FInoHandle {
    fn drop(&mut self) {
        FINO_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub fn fsid_alloc() -> FSIDHandle {
    FSIDHandle(FSID_ALLOCATOR.lock().alloc())
}

pub fn fino_alloc() -> FInoHandle {
    FInoHandle(FINO_ALLOCATOR.lock().alloc())
}

impl<T> KFile<T> {
    pub fn new(inode: Arc<T>, path: AbsolutePath, open_flags: OpenFlags) -> Self {
        let available = true;
        let fid = fino_alloc();
        Self {
            inner: Mutex::new(KFileInner {
                fid,
                offset: 0,
                inode,
                available,
                flags: open_flags,
                time_info: TimeInfo::empty(),
            }),
            path,
        }
    }
    pub fn name(&self) -> String {
        // AbsolutePath 中包含了文件名
        self.path.name()
    }
    // pub fn is_readable(&self) -> bool {
    //     self.open_flags.is_readable()
    // }
    // pub fn is_writable(&self) -> bool {
    //     self.open_flags.is_writable()
    // }
}

impl<T> KFS<T> {
    pub fn new(fs: Arc<RwLock<T>>, mount_path: AbsolutePath) -> Self {
        let id = fsid_alloc();
        Self { fs, id, mount_path }
    }

    pub fn id(&self) -> usize {
        self.id.get()
    }
}
