use crate::syscall::impls::Errno;

use super::File;

pub trait DeviceFile: File {
    fn get_id(&self) -> usize {
        unimplemented!();
    }
    fn ioctl(&self, _request: usize, _arg: usize) -> Result<isize, Errno> {
        unimplemented!();
    }
}

pub trait BlockFile: DeviceFile {
    fn read_block(&self, _block_id: usize, _buf: &mut [u8]) {
        unimplemented!();
    }
    fn write_block(&self, _block_id: usize, _buf: &[u8]) {
        unimplemented!();
    }
}

pub trait CharFile: DeviceFile {
    fn getchar(&mut self) -> u8 {
        unimplemented!();
    }
    fn putchar(&mut self, _ch: u8) {
        unimplemented!();
    }
}
