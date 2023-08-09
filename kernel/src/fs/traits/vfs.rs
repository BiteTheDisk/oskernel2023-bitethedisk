use crate::fs::Statfs;
use alloc::{string::String, sync::Arc};
use core::any::Any;
use path::AbsolutePath;

use super::File;
use crate::fs::OpenFlags;

pub trait VFS: Send + Sync + Any {
    fn mount_path(&self) -> AbsolutePath {
        panic!("no implement");
    }

    fn root_dir(&self, _mode: OpenFlags) -> Arc<dyn File> {
        panic!("no implement");
    }

    fn statfs(&self) -> Statfs {
        panic!("no implement");
    }

    fn list(&self, _path: &AbsolutePath) {
        panic!("no implement");
    }

    fn fsid(&self) -> usize {
        panic!("no implement");
    }

    fn name(&self) -> String {
        panic!("no implement");
    }
}
