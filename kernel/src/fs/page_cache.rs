// 使用 page cache 前相当于对于文件的读写是直接与 fat32 交互; 现在相当于在 kernel 与 fat32 之间加了一层缓存

use alloc::{
    collections::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::RwLock;

use crate::{
    consts::PAGE_SIZE,
    mm::{FilePage, MapPermission},
    syscall::impls::Errno,
};

use super::File;

pub struct PageCache {
    inode: Option<Weak<dyn File>>,
    // page number -> page
    pub pages: RwLock<BTreeMap<usize, Arc<FilePage>>>,
}

impl PageCache {
    pub fn new(inode: Arc<dyn File>) -> Self {
        Self {
            inode: Some(Arc::downgrade(&inode)),
            pages: RwLock::new(BTreeMap::new()),
        }
    }
    fn lookup(&self, offset: usize) -> Option<Arc<FilePage>> {
        self.pages.read().get(&(offset / PAGE_SIZE)).cloned()
    }
    pub fn insert(&self, offset: usize, page: FilePage) {
        debug_assert!(self
            .pages
            .write()
            .insert(offset / PAGE_SIZE, Arc::new(page))
            .is_none())
    }
    /// Get a page according to the given file offset
    pub fn get_page(
        &self,
        offset: usize,
        map_perm: Option<MapPermission>,
    ) -> Result<Arc<FilePage>, Errno> {
        // trace!("[PageCache]: get page at file offset {:#x}", offset);
        trace!("[PageCache]: get page at file offset {:#x}", offset);
        if let Some(page) = self.lookup(offset) {
            Ok(page)
        } else {
            let page = Arc::new(FilePage::new(
                map_perm.unwrap_or(MapPermission::R | MapPermission::W),
                offset,
                self.inode.as_ref().unwrap().upgrade().unwrap(),
            ));
            self.pages.write().insert(offset / PAGE_SIZE, page.clone());
            Ok(page)
        }
    }
}
