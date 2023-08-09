use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use fat32::BlockDevice;
use fat32::ATTR_DIRECTORY;
use spin::Mutex;

use crate::fs::dev::DevFS;
use crate::fs::proc::ProcFS;
use crate::fs::FSType;
use crate::fs::File;
use crate::fs::OpenFlags;
use crate::fs::Statfs;
use crate::fs::S_IFDIR;
use crate::fs::VFS;
use crate::fs::{CreateMode, Fat32};
use crate::syscall::impls::Errno;
use path::AbsolutePath;

use crate::drivers::BLOCK_DEVICE;

const MNT_MAXLEN: usize = 16;

pub struct MountTable {
    mnt_list: Vec<(String, AbsolutePath, FSType)>, // special(设备名), dir(挂载点), fstype(文件系统类型)
}

impl MountTable {
    pub fn new() -> Self {
        MountTable {
            mnt_list: Vec::new(),
        }
    }

    pub fn mount(&mut self, special: String, dir: String, fstype: String, flags: u32) -> isize {
        if self.mnt_list.len() == MNT_MAXLEN {
            return -1;
        }
        let dir_ = AbsolutePath::from_string(dir);

        // 已存在
        if self.mnt_list.iter().find(|&(_, d, _)| *d == dir_).is_some() {
            return 0;
        }

        // TODO
        _ = flags;

        let fstype_ = match fstype.as_str() {
            "fat32" | "vfat" => FSType::FAT32,
            // TODO
            _ => FSType::UNKNOWN,
        };

        self.mnt_list.push((special, dir_, fstype_));

        0
    }

    // unmount2: 移除挂载点
    pub fn umount2(&mut self, dir: String, flags: u32) -> isize {
        let len = self.mnt_list.len();
        let dir_ = AbsolutePath::from_string(dir);

        // TODO
        _ = flags;

        for i in 0..len {
            if self.mnt_list[i].1 == dir_ {
                self.mnt_list.remove(i);
                return 0;
            }
        }
        -1
    }

    // unmount: 移除设备
    #[allow(unused)]
    pub fn umount(&mut self, special: String, flags: u32) -> isize {
        let len = self.mnt_list.len();

        // TODO
        _ = flags;

        for i in 0..len {
            if self.mnt_list[i].0 == special {
                self.mnt_list.remove(i);
                return 0;
            }
        }
        -1
    }
}

pub fn open_fs(
    fstype: FSType,
    device: Arc<dyn BlockDevice>,
    mount_path: AbsolutePath,
) -> Result<Arc<dyn VFS>, Errno> {
    match fstype {
        FSType::FAT32 => {
            let fs = fat32::FileSystem::open(device);
            return Ok(Arc::new(Fat32::new(fs, mount_path)));
        }
        FSType::DEV => {
            return Ok(Arc::new(DevFS::init(mount_path)));
        }
        FSType::PROC => {
            return Ok(Arc::new(ProcFS::init(mount_path)));
        }
        _ => return Err(Errno::ENOENT),
    }
}

pub struct RootFS {
    pub rfs: Arc<dyn VFS>,
    pub mount_table: MountTable,
    pub vfs_map: BTreeMap<AbsolutePath, Arc<dyn VFS>>, // 挂载点(前缀) -> 文件系统
}

lazy_static! {
    pub static ref ROOT_FS: Mutex<RootFS> =
        Mutex::new(RootFS::new(FSType::FAT32, Arc::clone(&BLOCK_DEVICE)));
}

impl RootFS {
    pub fn new(fstype: FSType, device: Arc<dyn BlockDevice>) -> Self {
        let mount_path = AbsolutePath::from_str("/");
        let fs = open_fs(fstype, device, mount_path.clone()).unwrap();
        let mut root_fs = RootFS {
            rfs: Arc::clone(&fs),
            mount_table: MountTable::new(),
            vfs_map: BTreeMap::new(),
        };
        root_fs.vfs_map.insert(mount_path.clone(), Arc::clone(&fs));
        root_fs
    }

    pub fn statfs(&self) -> Statfs {
        self.rfs.statfs()
    }

    pub fn open(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        mode: CreateMode,
    ) -> Result<Arc<dyn File>, Errno> {
        let (path_, root_dir) = self.find_root(path, flags, mode).unwrap();
        root_dir.open(path_, flags, mode)
    }

    /// 找到对应的文件系统的根节点以及路径
    pub fn find_root(
        &self,
        path: AbsolutePath,
        flags: OpenFlags,
        _mode: CreateMode,
    ) -> Option<(AbsolutePath, Arc<dyn File>)> {
        // 根据 path 确定文件系统 -> 在 mount table 中匹配最长前缀(嵌套的文件系统)
        let mut size = 0;
        let mut root_dir = self.rfs.root_dir(flags);
        let mut prefix = AbsolutePath::from_str("/");
        for iter in self.mount_table.mnt_list.iter() {
            let (_, prefix_i, _) = iter;
            if path.start_with(prefix_i) {
                if size <= prefix_i.layer() {
                    // root layer = 0
                    size = prefix_i.layer();
                    let fs = self.vfs_map.get(prefix_i).unwrap();
                    // 获取对应文件系统的根节点
                    root_dir = fs.root_dir(flags); // TODO
                    prefix = prefix_i.clone();
                }
            }
        }
        let mut pathv = path.as_vec_str();
        let pathv2 = prefix.as_vec_str();
        pathv.drain(0..pathv2.len());

        Some((AbsolutePath::from_vec_str(pathv), root_dir))
    }
}

pub fn list_apps(path: AbsolutePath) {
    fn ls(path: AbsolutePath, layer: usize, root_dir: Arc<dyn File>) {
        let dir = root_dir
            .open(path.clone(), OpenFlags::empty(), CreateMode::empty())
            .unwrap();
        for app in dir.ls_with_attr() {
            // 不打印initproc, 事实上它也在task::new之后删除了
            if layer == 0 && app.0 == "initproc" {
                continue;
            }
            let app_path = path.cd(app.0.clone());
            if app.1 & ATTR_DIRECTORY as u32 == 0 && app.1 != S_IFDIR {
                // 如果不是目录
                for _ in 0..layer {
                    print!("   ");
                }
                crate::println!("{}", app.0);
            } else if app.0 != "." && app.0 != ".." {
                // 目录
                for _ in 0..layer {
                    crate::print!("   ");
                }
                crate::println!("{}/", app.0);
                ls(app_path.clone(), layer + 1, root_dir.clone());
            }
        }
    }

    for fs in ROOT_FS.lock().vfs_map.values() {
        let root_dir = fs.root_dir(OpenFlags::empty());
        let prefix = fs.mount_path();
        let layer = prefix.layer();
        for i in 0..layer {
            for _ in 0..i {
                crate::print!("   ");
            }
            println!("{}/", prefix.index(i));
        }
        ls(path.clone(), layer, root_dir.clone());
    }
}

// work_path 绝对路径

pub fn chdir(path: AbsolutePath) -> bool {
    let root_dir = ROOT_FS.lock().rfs.root_dir(OpenFlags::empty());
    // TODO use open
    if root_dir.find(path).is_some() {
        true
    } else {
        false
    }
}

pub fn open(
    path: AbsolutePath,
    flags: OpenFlags,
    mode: CreateMode,
) -> Result<Arc<dyn File>, Errno> {
    ROOT_FS.lock().open(path, flags, mode)
}

pub fn mount(special: String, dir: String, fstype: String, flags: u32) -> isize {
    let mut root_fs = ROOT_FS.lock();
    let mount_path = AbsolutePath::from_string(dir.clone());

    // TODO 具体硬件
    let device = match special {
        _ => Arc::clone(&BLOCK_DEVICE),
    };

    // TODO mount_table mount 函数重复处理 fstype
    let fstype_ = match fstype.as_str() {
        "fat32" => FSType::FAT32,
        "devfs" | "dev" => FSType::DEV,
        "procfs" | "proc" => FSType::PROC,
        _ => FSType::FAT32,
    };

    let fs = open_fs(fstype_, device, mount_path.clone()).unwrap();
    root_fs.vfs_map.insert(mount_path.clone(), Arc::clone(&fs));

    root_fs
        .mount_table
        .mount(special, dir.clone(), fstype, flags)
}

// TODO 优化 string absolutepath 关于clone的问题

pub fn umount2(dir: String, flags: u32) -> isize {
    let mut root_fs = ROOT_FS.lock();
    let mount_path = AbsolutePath::from_string(dir.clone());

    root_fs.vfs_map.remove(&mount_path.clone());

    root_fs.mount_table.umount2(dir.clone(), flags)
}

#[allow(unused)]
pub fn umount(special: String, flags: u32) -> isize {
    let mut root_fs = ROOT_FS.lock();

    let mut mount_path = AbsolutePath::from_str("/");
    let mut flag = false;

    for iter in root_fs.mount_table.mnt_list.iter() {
        let (dev, path, _) = iter;
        if dev == &special {
            mount_path = path.clone();
            flag = true;
            break;
        }
    }

    if flag == false {
        return -1;
    }

    root_fs.vfs_map.remove(&mount_path);

    root_fs.mount_table.umount(special, flags)
}
