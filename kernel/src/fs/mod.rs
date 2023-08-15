//! Kernel file system
//!
//! The kernel uniformly borrows the VirtFile provided by the fat32 file system
//! as the object for the kernel to operate files.
//!
//! Due to the coupling between our kernel's files and the FAT32 file system
//! (the kernel files are based on FAT32), for certain tests that require specific
//! files/directories to be present in the kernel, they must be created in advance
//! at this location.
//! A more reasonable solution would be to implement a tempfs within the kernel.
//! However, as we are about to enter the second stage of the national competition,
//! there is currently no time to improve the kernel file system.
//! If future participating teams refer to the code implementation of our file system,
//! we recommend looking at the implementation of the file system in TitanixOS,
//! which was developed by a team from the same competition.
//! In simple terms, TitanixOS implements most of its files within the kernel instead
//! of relying on the FAT32 file system. This allows for significantly faster
//! execution speed during testing in TitanixOS.
//! TitanixOS seems to only read test files/programs from FAT32 filesystems

mod fat32;
mod file;
mod mount;
mod page_cache;
mod pipe;
mod stdio;
pub use self::fat32::*;
pub use file::*;
pub use mount::*;
pub use page_cache::*;
pub use path::*;
pub use pipe::*;
pub use stdio::*;

mod page;
pub use page::*;

use alloc::string::ToString;
use nix::{CreateMode, OpenFlags};
use path::AbsolutePath;
use sync_cell::SyncRefCell;

pub fn init() {
    // Due to the coupling between our kernel's files and the FAT32 file system
    // (the kernel files are based on FAT32), for certain tests that require specific
    // files/directories to be present in the kernel, they must be created in advance
    // at this location.
    open(
        "/proc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/tmp".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev".into(),
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
        "/dev/misc".into(),
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev/shm".into(),
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
    open("/dev/null".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open("/dev/zero".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open(
        "/proc/mounts".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/proc/meminfo".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/dev/misc/rtc".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();
    open(
        "/var/tmp/lmbench".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();

    // for sys_clock_getres
    open(
        "/dev/cpu_dma_latency".into(),
        OpenFlags::O_CREATE,
        CreateMode::empty(),
    )
    .unwrap();

    open("/dev/tty".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();
    open("/lat_sig".into(), OpenFlags::O_CREATE, CreateMode::empty()).unwrap();

    println!("===+ Files Loaded +===");
    list_apps(AbsolutePath::from_string("/".to_string()));
    println!("===+==============+===");
}

static INO_ALLOCATOR: SyncRefCell<Allocator> = SyncRefCell::new(Allocator::new());
struct Allocator {
    current: u64,
}
impl Allocator {
    pub const fn new() -> Self {
        Allocator { current: 0 }
    }
    fn fetch_add(&mut self) -> u64 {
        let id = self.current;
        self.current += 1;
        id
    }
    pub fn alloc(&mut self) -> u64 {
        self.fetch_add()
    }
}
pub fn ino_alloc() -> u64 {
    INO_ALLOCATOR.borrow_mut().alloc()
}
