//! 目录项

use alloc::string::String;
use alloc::string::ToString;

use super::{S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFREG, S_IFSOCK};

pub const NAME_LIMIT: usize = 64;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum FileType {
    UNKNOWN = 0,
    FIFOFile = 1,
    CharDevice = 2,
    Directory = 4,
    BlockDevice = 6,
    RegularFile = 8,
    LinkFile = 10,
    SocketFile = 12,
}

impl FileType {
    pub fn from_file_mode(mode: u32) -> Self {
        match mode {
            S_IFIFO => Self::FIFOFile,
            S_IFCHR => Self::CharDevice,
            S_IFDIR => Self::Directory,
            S_IFBLK => Self::BlockDevice,
            S_IFREG => Self::RegularFile,
            S_IFLNK => Self::LinkFile,
            S_IFSOCK => Self::SocketFile,
            _ => Self::UNKNOWN,
        }
    }

    pub fn to_file_mode(&self) -> u32 {
        match self {
            Self::FIFOFile => S_IFIFO,
            Self::CharDevice => S_IFCHR,
            Self::Directory => S_IFDIR,
            Self::BlockDevice => S_IFBLK,
            Self::RegularFile => S_IFREG,
            Self::LinkFile => S_IFLNK,
            Self::SocketFile => S_IFSOCK,
            _ => 0,
        }
    }

    pub fn from_u8(file_type: u8) -> Self {
        match file_type {
            1 => Self::FIFOFile,
            2 => Self::CharDevice,
            4 => Self::Directory,
            6 => Self::BlockDevice,
            8 => Self::RegularFile,
            10 => Self::LinkFile,
            12 => Self::SocketFile,
            _ => Self::UNKNOWN,
        }
    }
}

/// 存储目录中的文件信息
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dirent {
    d_ino: usize,             // 索引节点号
    d_off: isize,             // 从 0 开始到下一个 dirent 的偏移
    d_reclen: u16,            // 当前 dirent 的长度
    d_type: u8,               // 文件类型
    d_name: [u8; NAME_LIMIT], // 文件名
}

impl Dirent {
    pub fn empty() -> Self {
        Self {
            d_ino: 0,
            d_off: 0,
            d_reclen: core::mem::size_of::<Self>() as u16,
            d_type: 0,
            d_name: [0; NAME_LIMIT],
        }
    }

    pub fn init(&mut self, name: &str, offset: isize, first_clu: usize, file_type: FileType) {
        self.d_ino = first_clu;
        self.d_off = offset;
        self.d_type = file_type as u8;
        self.fill_name(name);
    }

    pub fn new(name: &str, offset: isize, first_clu: usize, file_type: FileType) -> Self {
        let mut dirent = Self::empty();
        dirent.init(name, offset, first_clu, file_type);
        dirent
    }

    fn fill_name(&mut self, name: &str) {
        let len = name.len().min(NAME_LIMIT);
        let name_bytes = name.as_bytes();
        for i in 0..len {
            self.d_name[i] = name_bytes[i];
        }
        self.d_name[len] = 0;
    }

    fn name(&self) -> String {
        let mut len = 0;
        while len < NAME_LIMIT && self.d_name[len] != 0 {
            len += 1;
        }
        String::from_utf8_lossy(&self.d_name[..len]).to_string()
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    /// 获取文件名 与 文件类型 (S_IFDIR、S_IFREG、S_IFLNK....
    pub fn info(&self) -> (String, u32) {
        (self.name(), FileType::from_u8(self.d_type).to_file_mode())
    }
}
