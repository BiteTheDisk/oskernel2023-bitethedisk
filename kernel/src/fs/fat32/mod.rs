use crate::fs::KFS;

pub type Fat32 = KFS<fat32::FileSystem>;

use crate::fs::stat::{S_IFCHR, S_IFDIR, S_IFREG};
use crate::fs::File;
use crate::fs::KFile;
use crate::fs::Kstat;
use crate::fs::OpenFlags;
use crate::fs::{CreateMode, FileType};
use crate::return_errno;
use crate::syscall::errno::Errno;
use path::AbsolutePath;

use crate::mm::UserBuffer;
use alloc::{string::String, sync::Arc, vec::Vec};
use fat32::{Dir, VirtFileType, ATTR_ARCHIVE, ATTR_DIRECTORY, BLOCK_SIZE};

use crate::fs::Dirent;
use crate::fs::FSType;
use crate::fs::VFS;

use alloc::string::ToString;

use crate::fs::Statfs;

use fat32::root;

use super::TimeInfo;

pub type Fat32File = KFile<fat32::VirtFile>;

const BIG_FILE_SIZE: usize = 96 * 4096;

impl File for Fat32File {
    // remove this
    // TODO lzm
    fn read_all(&self) -> Vec<u8> {
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = vec![];
        let mut inner = self.inner.lock();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }

    fn read(&self, offset: usize, len: usize) -> Vec<u8> {
        let inner = self.inner.lock();
        let mut len = len;
        let mut offset_ = offset;
        let mut buffer = [0u8; BLOCK_SIZE];
        let mut v: Vec<u8> = Vec::new();
        if len >= BIG_FILE_SIZE {
            // 防止 v 占用空间过度扩大
            v.reserve(BIG_FILE_SIZE);
        }
        loop {
            let read_size = inner.inode.read_at(offset_, &mut buffer);
            if read_size == 0 {
                break;
            }
            offset_ += read_size;
            v.extend_from_slice(&buffer[..read_size.min(len)]);
            if len > read_size {
                len -= read_size;
            } else {
                break;
            }
        }

        v
    }

    // write_all
    // TODO lzm
    fn write(&self, str_vec: &Vec<u8>) -> usize {
        let mut inner = self.inner.lock();
        let mut remain = str_vec.len();
        let mut base = 0;

        loop {
            let len = remain.min(512);
            inner
                .inode
                .write_at(inner.offset, &str_vec.as_slice()[base..base + len]);
            inner.offset += len;
            base += len;
            remain -= len;
            if remain == 0 {
                break;
            }
        }
        base
    }

    fn is_dir(&self) -> bool {
        let inner = self.inner.lock();
        inner.inode.is_dir()
    }

    fn name(&self) -> String {
        self.name()
    }

    fn delete(&self) {
        let inner = self.inner.lock();
        inner.inode.clear();
    }

    fn delete_direntry(&self) {
        let inner = self.inner.lock();
        inner.inode.clear_direntry();
    }

    fn file_size(&self) -> usize {
        let inner = self.inner.lock();
        inner.inode.file_size() as usize
    }

    fn path(&self) -> AbsolutePath {
        self.path.clone()
    }

    fn seek(&self, _pos: usize) {
        let mut inner = self.inner.lock();
        inner.offset = _pos;
    }

    fn readable(&self) -> bool {
        let inner = self.inner.lock();
        inner.flags.is_readable()
    }

    fn writable(&self) -> bool {
        let inner = self.inner.lock();
        inner.flags.is_writable()
    }

    fn available(&self) -> bool {
        let inner = self.inner.lock();
        inner.available
    }

    fn read_to_buf(&self, mut buf: UserBuffer) -> usize {
        let offset = self.inner.lock().offset;
        let file_size = self.file_size();
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;

        if file_size == 0 {
            // TODO lzm
            if self.name() == "zero" {
                buf.write_zeros();
            }
            return 0;
        }

        if offset >= file_size {
            return 0;
        }

        // 这边要使用 iter_mut(), 因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn pread(&self, mut buf: UserBuffer, offset: usize) -> usize {
        let inner = self.inner.lock();
        let mut index = offset;
        let file_size = inner.inode.file_size();

        let mut total_read_size = 0usize;

        // TODO
        //  lzm
        if file_size == 0 {
            if self.name() == "zero" {
                buf.write_zeros();
            }
            return 0;
        }

        if offset >= file_size {
            return 0;
        }

        // 这边要使用 iter_mut(), 因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(index, *slice);
            if read_size == 0 {
                break;
            }
            index += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn write_from_buf(&self, buf: UserBuffer) -> usize {
        let mut total_write_size = 0usize;
        let filesize = self.file_size();
        let mut inner = self.inner.lock();
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(filesize, *slice);
                inner.offset += write_size;
                total_write_size += write_size;
            }
        } else {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(inner.offset, *slice);
                assert_eq!(write_size, slice.len());
                inner.offset += write_size;
                total_write_size += write_size;
            }
        }
        total_write_size
    }

    fn pwrite(&self, buf: UserBuffer, offset: usize) -> usize {
        let inner = self.inner.lock();
        let mut index = offset;
        let file_size = inner.inode.file_size();

        let mut total_write_size = 0usize;
        if inner.flags.contains(OpenFlags::O_APPEND) {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(file_size + total_write_size, *slice);
                total_write_size += write_size;
            }
        } else {
            for slice in buf.buffers.iter() {
                let write_size = inner.inode.write_at(index, *slice);
                assert_eq!(write_size, slice.len());
                index += write_size;
                total_write_size += write_size;
            }
        }
        total_write_size
    }

    fn set_time(&self, time_info: TimeInfo) {
        let mut inner = self.inner.lock();
        let time_lock = &mut inner.time_info;
        // 根据测例改动
        if time_info.mtime < time_lock.mtime {
            time_lock.atime = time_info.atime;
            time_lock.ctime = time_info.ctime;
        } else {
            *time_lock = time_info;
        }
    }

    fn set_cloexec(&self) {
        let mut inner = self.inner.lock();
        inner.flags |= OpenFlags::O_CLOEXEC;
        inner.available = false;
    }

    fn set_flags(&self, flag: OpenFlags) {
        let mut inner = self.inner.lock();
        inner.flags = flag;
    }

    fn fstat(&self, kstat: &mut Kstat) {
        let inner = self.inner.lock();
        let ino = inner.fid.get();
        let vfile = inner.inode.clone();
        let mut st_mode = 0;
        _ = st_mode;
        let (st_size, st_blksize, st_blocks, is_dir, time) = vfile.stat();

        if is_dir {
            st_mode = S_IFDIR;
        } else {
            st_mode = S_IFREG;
        }
        if vfile.name() == "null"
            || vfile.name() == "NULL"
            || vfile.name() == "zero"
            || vfile.name() == "ZERO"
        {
            st_mode = S_IFCHR;
        }
        let time_info = inner.time_info;
        let atime = time_info.atime;
        let mtime = time_info.mtime;
        let ctime = time_info.ctime;
        kstat.init(
            st_size as i64,
            st_blksize as i32,
            st_blocks as u64,
            st_mode as u32,
            ino as u64,
            atime as i64,
            mtime as i64,
            ctime as i64,
        );
    }

    fn offset(&self) -> usize {
        let inner = self.inner.lock();
        inner.offset
    }

    //
    // Dir implementation
    //
    fn all_dirents(&self) -> Option<Vec<Dirent>> {
        if !self.is_dir() {
            return None;
        }

        let mut dirents = Vec::new();
        let inner = self.inner.lock();
        let mut fp = 0usize;

        loop {
            if let Some((name, offset, first_cluster, attr)) = inner.inode.dir_info(fp) {
                let file_type = match attr as u8 {
                    ATTR_DIRECTORY => FileType::Directory,
                    ATTR_ARCHIVE => FileType::RegularFile,
                    _ => FileType::UNKNOWN,
                };

                dirents.push(Dirent::new(
                    name.as_str(),
                    offset as isize,
                    first_cluster as usize,
                    file_type,
                ));
                fp = offset;
            } else {
                break;
            }
        }

        Some(dirents)
    }

    fn dirent(&self, dirent: &mut Dirent) -> isize {
        if !self.is_dir() {
            return -1;
        }

        let mut inner = self.inner.lock();
        let offset = inner.offset as u32;
        if let Some((name, offset, first_cluster, attr)) = inner.inode.dir_info(offset as usize) {
            let file_type = match attr as u8 {
                ATTR_DIRECTORY => FileType::Directory,
                ATTR_ARCHIVE => FileType::RegularFile,
                _ => FileType::UNKNOWN,
            };

            dirent.init(
                name.as_str(),
                offset as isize,
                first_cluster as usize,
                file_type,
            );
            // TODO 为什么要改变 offset
            inner.offset = offset as usize;
            // return size of Dirent as read size
            core::mem::size_of::<Dirent>() as isize
        } else {
            -1
        }
    }

    fn find(&self, path: AbsolutePath) -> Option<Arc<dyn File>> {
        if !self.is_dir() {
            return None;
        }
        let inner = self.inner.lock();
        let pathv = path.as_vec_str().clone();
        let res = inner.inode.find(pathv);
        match res {
            Ok(inode) => {
                let open_flags = OpenFlags::empty();
                Some(Arc::new(Fat32File::new(inode, path, open_flags)))
            }
            Err(_) => None,
        }
    }

    fn open(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        match self.open_(path, flags, mode) {
            Ok(fat32_file) => Ok(fat32_file),
            Err(err) => Err(err),
        }
    }

    fn ls(&self) -> Vec<String> {
        todo!()
    }

    fn ls_with_attr(&self) -> Vec<(String, u32)> {
        let inner = self.inner.lock();
        inner.inode.ls_with_attr().unwrap()
    }

    fn rename(&self, new_path: AbsolutePath, flags: OpenFlags) {
        // duplicate a new file, and set file cluster and file size
        let inner = self.inner.lock();
        // check file exits
        let new_file = self.open_(new_path, flags, CreateMode::empty()).unwrap();
        let new_inner = new_file.inner.lock();
        let first_cluster = inner.inode.first_cluster();
        let file_size = inner.inode.file_size();

        new_inner.inode.set_first_cluster(first_cluster);
        new_inner.inode.set_file_size(file_size);

        drop(inner);
        // clear old direntry
        self.delete_direntry();
    }

    fn ino(&self) -> usize {
        let inner = self.inner.lock();
        inner.fid.get()
    }
}

impl Fat32File {
    fn open_(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        _mode: CreateMode,
    ) -> Result<Arc<Self>, Errno> {
        if !self.is_dir() {
            return Err(Errno::ENOTDIR);
        }
        let inner = self.inner.lock();
        let pathv = path.as_vec_str().clone();

        // 创建文件
        if flags.contains(OpenFlags::O_CREATE) {
            let res = inner.inode.find(pathv.clone());
            match res {
                Ok(fat32_vf) => {
                    fat32_vf.clear_content(); // 清空文件内容

                    // TODO 测试一下名字
                    Ok(Arc::new(Fat32File::new(fat32_vf, path, flags)))
                }
                Err(_) => {
                    let mut create_type = VirtFileType::File;
                    if flags.contains(OpenFlags::O_DIRECTROY) {
                        create_type = VirtFileType::Dir;
                    }

                    let parent_path = path.clone().parent();
                    let name = path.name();
                    match inner.inode.find(parent_path.as_vec_str()) {
                        Ok(parent_vf) => match parent_vf.create(name.as_str(), create_type) {
                            Ok(fat32_vf) => {
                                Ok(Arc::new(Fat32File::new(Arc::new(fat32_vf), path, flags)))
                            }
                            Err(_) => Err(Errno::UNCLEAR),
                        },
                        Err(_) => {
                            return_errno!(Errno::ENOENT, "parent path not exist path:{:?}", path)
                        }
                    }
                }
            }
        } else {
            let res = inner.inode.find(pathv);
            match res {
                Ok(fat32_vf) => {
                    if flags.contains(OpenFlags::O_TRUNC) {
                        fat32_vf.clear_content();
                    }
                    Ok(Arc::new(Fat32File::new(fat32_vf, path, flags)))
                }
                Err(_) => return_errno!(Errno::ENOENT, "no such file or path:{:?}", path),
            }
        }
    }

    pub fn delete_direntry(&self) {
        let inner = self.inner.lock();
        inner.inode.clear_direntry();
    }

    fn truncate(&self, new_length: usize) {
        let inner = self.inner.lock();
        inner.inode.modify_size(new_length);
    }
}

impl VFS for Fat32 {
    fn root_dir(&self, mode: OpenFlags) -> Arc<dyn File> {
        let inner = Arc::new(root(self.fs.clone()));
        let path = AbsolutePath::from_str("/");

        Arc::new(Fat32File::new(inner, path, mode))
    }

    fn mount_path(&self) -> AbsolutePath {
        self.mount_path.clone()
    }

    fn statfs(&self) -> Statfs {
        /* 为了方便, 就不获取空闲块s数量了 */
        let free_clus_cnt = self.fs.read().free_cluster_cnt() as u64;
        let clus_cnt = self.fs.read().total_cluster_cnt() as u64;
        let spc = self.fs.read().sector_pre_cluster() as u64;
        let sec_size = self.fs.read().sector_size() as u64;
        let mut statfs = Statfs::default();

        statfs.f_bsize = sec_size as u64;
        statfs.f_frsize = spc * sec_size as u64;
        statfs.f_blocks = clus_cnt * spc as u64;
        statfs.f_bfree = free_clus_cnt * spc as u64;
        statfs.f_bavail = free_clus_cnt * spc as u64;

        statfs.f_files = free_clus_cnt as u64;
        statfs.f_ffree = free_clus_cnt as u64;
        statfs.f_fsid = free_clus_cnt as u64;
        statfs.f_flag = free_clus_cnt as u64;

        statfs.f_type = FSType::FAT32 as u64;

        statfs
    }

    fn fsid(&self) -> usize {
        self.id()
    }

    fn name(&self) -> String {
        "fat32".to_string()
    }
}
