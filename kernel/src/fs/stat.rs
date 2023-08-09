#![allow(unused)]

use super::FSType;

pub const S_IFDIR: u32 = 0o0040000;
pub const S_IFCHR: u32 = 0o0020000;
pub const S_IFBLK: u32 = 0o0060000;
pub const S_IFREG: u32 = 0o0100000;
pub const S_IFIFO: u32 = 0o0010000;
pub const S_IFLNK: u32 = 0o0120000;
pub const S_IFSOCK: u32 = 0o0140000;

#[repr(C)]
#[derive(Debug, Default)]
pub struct Kstat {
    pub st_dev: u64,   // 包含文件的设备 ID
    pub st_ino: u64,   // 索引节点号
    pub st_mode: u32,  // 文件类型和模式
    pub st_nlink: u32, // 硬链接数
    pub st_uid: u32,   // 所有者的用户 ID
    pub st_gid: u32,   // 所有者的组 ID
    pub st_rdev: u64,  // 设备 ID(如果是特殊文件)
    __pad: u64,
    pub st_size: i64,    // 总大小, 以字节为单位
    pub st_blksize: i32, // 文件系统 I/O 的块大小
    __pad2: i32,
    pub st_blocks: u64,     // 分配的 512B 块数
    pub st_atime_sec: i64,  // 上次访问时间
    pub st_atime_nsec: i64, // 上次访问时间(纳秒精度)
    pub st_mtime_sec: i64,  // 上次修改时间
    pub st_mtime_nsec: i64, // 上次修改时间(纳秒精度)
    pub st_ctime_sec: i64,  // 上次状态变化的时间
    pub st_ctime_nsec: i64, // 上次状态变化的时间(纳秒精度)
    __unused: [u32; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct TimeInfo {
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
}

impl TimeInfo {
    pub fn empty() -> Self {
        Self {
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }
}

impl Kstat {
    pub fn empty() -> Self {
        Self {
            st_dev: 0,
            st_ino: 0 as u64,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 0,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0,
            st_atime_sec: 0,
            st_atime_nsec: 0,
            st_mtime_sec: 0,
            st_mtime_nsec: 0,
            st_ctime_sec: 0,
            st_ctime_nsec: 0,
            __unused: [0; 2],
        }
    }
    pub fn init(
        &mut self,
        st_size: i64,
        st_blksize: i32,
        st_blocks: u64,
        st_mode: u32,
        st_ino: u64,
        st_atime_sec: i64,
        st_mtime_sec: i64,
        st_ctime_sec: i64,
    ) {
        self.st_nlink = 1;
        self.st_ino = st_ino;
        self.st_size = st_size;
        self.st_blksize = st_blksize;
        self.st_blocks = st_blocks;
        self.st_mode = st_mode;
        self.st_atime_sec = st_atime_sec;
        self.st_mtime_sec = st_mtime_sec;
        self.st_ctime_sec = st_ctime_sec;
    }
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Statfs {
    pub f_type: u64,
    pub f_bsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: u64,
    pub f_namelen: u64,
    pub f_frsize: u64,
    pub f_flag: u64,
    pub f_spare: [u64; 4],
}

impl Statfs {
    pub fn new() -> Self {
        Self {
            f_type: 1,
            f_bsize: 512,
            f_blocks: 12345,
            f_bfree: 1234,
            f_bavail: 123,
            f_files: 1000,
            f_ffree: 100,
            f_fsid: 1,
            f_namelen: 123,
            f_frsize: 4096,
            f_flag: 123,
            f_spare: [0; 4],
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

// TODO 替换 timeinfo
pub struct InodeTime {
    create_time: u64,
    access_time: u64,
    modify_time: u64,
}
impl InodeTime {
    /// Set the inode time's create time.
    pub fn set_create_time(&mut self, create_time: u64) {
        self.create_time = create_time;
    }

    /// Get a reference to the inode time's create time.
    pub fn create_time(&self) -> &u64 {
        &self.create_time
    }

    /// Set the inode time's access time.
    pub fn set_access_time(&mut self, access_time: u64) {
        self.access_time = access_time;
    }

    /// Get a reference to the inode time's access time.
    pub fn access_time(&self) -> &u64 {
        &self.access_time
    }

    /// Set the inode time's modify time.
    pub fn set_modify_time(&mut self, modify_time: u64) {
        self.modify_time = modify_time;
    }

    /// Get a reference to the inode time's modify time.
    pub fn modify_time(&self) -> &u64 {
        &self.modify_time
    }
}
