// 定义一份打开文件的标志
bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct OpenFlags: u32 {
        const O_RDONLY    = 0;       // 只读
        const O_WRONLY    = 1 << 0;  // 只写
        const O_RDWR      = 1 << 1;  // 读写
        const O_CREATE    = 1 << 6;  // 创建
        const O_EXCL      = 1 << 7;  // 排他
        const O_TRUNC     = 1 << 9;  // 截断
        const O_APPEND    = 1 << 10; // 追加
        const O_NONBLOCK  = 1 << 11; // 非阻塞
        const O_LARGEFILE = 1 << 15; // 大文件
        const O_DIRECTROY = 1 << 16; // 目录
        const O_NOFOLLOW  = 1 << 17; // 不跟随
        const O_CLOEXEC   = 1 << 19; // 关闭执行
    }

    /// 用户组读写权限
    #[derive(Debug, Clone, Copy)]
    pub struct CreateMode: u32 {
        const S_ISUID  = 0o4000;
        const S_ISGID  = 0o2000;
        const S_ISVTX  = 0o1000;

        const S_IRWXU  = 0o700;
        const S_IRUSR  = 0o400;
        const S_IWUSR  = 0o200;
        const S_IXUSR  = 0o100;

        const S_IRWXG  = 0o070;
        const S_IRGRP  = 0o040;
        const S_IWGRP  = 0o020;
        const S_IXGRP  = 0o010;

        const S_IRWXO  = 0o007;
        const S_IROTH  = 0o004;
        const S_IWOTH  = 0o002;
        const S_IXOTH  = 0o001;
    }
}

impl OpenFlags {
    pub fn is_readable(&self) -> bool {
        self.contains(Self::O_RDONLY) || self.contains(Self::O_RDWR)
    }

    pub fn is_writable(&self) -> bool {
        self.contains(Self::O_WRONLY) || self.contains(Self::O_RDWR)
    }
}
