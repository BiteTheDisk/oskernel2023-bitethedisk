use core::fmt::{self, Debug, Formatter};

use alloc::{string::String, sync::Arc, vec::Vec};

use crate::mm::UserBuffer;

use crate::fs::{AbsolutePath, Dirent, Kstat, OpenFlags};
use crate::fs::{CreateMode, TimeInfo};
use crate::syscall::impls::Errno;

pub trait File: Send + Sync {
    fn readable(&self) -> bool {
        panic!("{} not implement readable", self.name());
    }

    fn writable(&self) -> bool {
        panic!("{} not implement writable", self.name());
    }

    fn available(&self) -> bool {
        panic!("{} not implement available", self.name());
    }

    /// read 指的是从文件中读取数据放到缓冲区中, 最多将缓冲区填满, 并返回实际读取的字节数
    fn read_to_buf(&self, mut buf: UserBuffer) -> usize {
        panic!("{} not implement read_to_buf", self.name());
    }

    /// 将缓冲区中的数据写入文件, 最多将缓冲区中的数据全部写入, 并返回直接写入的字节数
    fn write_from_buf(&self, buf: UserBuffer) -> usize {
        panic!("{} not implement write_from_buf", self.name());
    }

    fn pread(&self, _buf: UserBuffer, _offset: usize) -> usize {
        panic!("{} not implement pread", self.name());
    }

    fn pwrite(&self, _buf: UserBuffer, _offset: usize) -> usize {
        panic!("{} not implement pwrite", self.name());
    }

    // TODO seek mode
    /// 设置文件的读写位置
    fn seek(&self, _pos: usize) {
        panic!("{} not implement seek", self.name());
    }

    /// 获取文件的读写位置
    fn offset(&self) -> usize {
        panic!("{} not implement offset", self.name());
    }

    fn name(&self) -> String {
        panic!("file not implement name");
    }

    /// 获取文件的状态信息
    fn fstat(&self, _kstat: &mut Kstat) {
        panic!("{} not implement fstat", self.name());
    }

    fn set_time(&self, _time_info: TimeInfo) {
        panic!("{} not implement set_time", self.name());
    }

    fn get_time(&self) -> TimeInfo {
        panic!("{} not implement get_time", self.name());
    }

    fn set_flags(&self, _flag: OpenFlags) {
        panic!("{} not implement set_open_flags", self.name());
    }

    /// 设置文件不可获取(正在被使用)
    fn set_cloexec(&self) {
        panic!("{} not implement set_cloexec", self.name());
    }

    fn read_all(&self) -> Vec<u8> {
        panic!("{} not implement read_all", self.name());
    }

    fn read(&self, _offset: usize, _len: usize) -> Vec<u8> {
        panic!("{} not implement read", self.name());
    }

    fn write(&self, _data: &Vec<u8>) -> usize {
        panic!("{} not implement write", self.name());
    }

    fn file_size(&self) -> usize {
        panic!("{} not implement file_size", self.name());
    }

    fn delete(&self) {
        panic!("{} not implement delete", self.name());
    }

    fn delete_direntry(&self) {
        panic!("{} not implement delete_direntry", self.name());
    }

    fn path(&self) -> AbsolutePath {
        panic!("{} not implement path", self.name());
    }

    fn is_dir(&self) -> bool {
        panic!("{} not implement is_dir", self.name());
    }

    //
    // For Dir
    //
    fn all_dirents(&self) -> Option<Vec<Dirent>> {
        panic!("{} not implement get_dirent", self.name());
    }

    fn dirent(&self, _dirent: &mut Dirent) -> isize {
        panic!("{} not implement dirent", self.name());
    }

    fn find(&self, _path: AbsolutePath) -> Option<Arc<dyn File>> {
        panic!("{} not implement find", self.name());
    }

    fn open(
        &self,
        _path: AbsolutePath,
        _flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        panic!("{} not implement open_dir", self.name());
    }

    fn remove(&self, _path: AbsolutePath) -> isize {
        panic!("{} not implement remove", self.name());
    }

    fn ls(&self) -> Vec<String> {
        panic!("{} not implement ls_with_attr", self.name());
    }

    fn ls_with_attr(&self) -> Vec<(String, u32)> {
        panic!("{} not implement ls_with_attr", self.name());
    }

    fn rename(&self, _new_path: AbsolutePath, _flags: OpenFlags) {
        panic!("{} not implement rename", self.name());
    }

    // for kstat.st_ino
    fn ino(&self) -> usize {
        panic!("{} not implement ino", self.name());
    }

    fn truncate(&self, _new_length: usize) {
        panic!("{} not implement truncate", self.name());
    }

    fn r_ready(&self) -> bool {
        true
    }
    fn w_ready(&self) -> bool {
        true
    }
}

impl Debug for dyn File + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "name:{}", self.name())
    }
}
