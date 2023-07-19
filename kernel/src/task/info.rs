//! 系统信息模块

use spin::Mutex;

use crate::timer::TimeVal;
pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

lazy_static! {
    pub static ref UTSNAME: Mutex<Utsname> = Mutex::new(Utsname::new());
}

impl Utsname {
    pub fn new() -> Self {
        Self {
            sysname: Utsname::str2u8("Linux"),
            nodename: Utsname::str2u8("ubuntu"),
            release: Utsname::str2u8("5.0"),
            version: Utsname::str2u8("5.13"),
            machine: Utsname::str2u8("riscv64"),
            domainname: Utsname::str2u8("Jeremy_test"),
        }
    }

    pub fn str2u8(str: &str) -> [u8; 65] {
        let mut arr: [u8; 65] = [0; 65];
        let cstr = str.as_bytes();
        let len = str.len();
        for i in 0..len {
            arr[i] = cstr[i];
        }
        arr
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

bitflags! {
    pub struct CloneFlags: usize{
        const SIGCHLD = 17; // 实际没有这个标志位
    }
}

#[allow(unused)]
pub struct RUsage {
    pub ru_utime: TimeVal, /* user CPU time used */
    pub ru_stime: TimeVal, /* system CPU time used */
    ru_maxrss: isize,      // NOT IMPLEMENTED /* maximum resident set size */
    ru_ixrss: isize,       // NOT IMPLEMENTED /* integral shared memory size */
    ru_idrss: isize,       // NOT IMPLEMENTED /* integral unshared data size */
    ru_isrss: isize,       // NOT IMPLEMENTED /* integral unshared stack size */
    ru_minflt: isize,      // NOT IMPLEMENTED /* page reclaims (soft page faults) */
    ru_majflt: isize,      // NOT IMPLEMENTED /* page faults (hard page faults) */
    ru_nswap: isize,       // NOT IMPLEMENTED /* swaps */
    ru_inblock: isize,     // NOT IMPLEMENTED /* block input operations */
    ru_oublock: isize,     // NOT IMPLEMENTED /* block output operations */
    ru_msgsnd: isize,      // NOT IMPLEMENTED /* IPC messages sent */
    ru_msgrcv: isize,      // NOT IMPLEMENTED /* IPC messages received */
    ru_nsignals: isize,    // NOT IMPLEMENTED /* signals received */
    ru_nvcsw: isize,       // NOT IMPLEMENTED /* voluntary context switches */
    ru_nivcsw: isize,      // NOT IMPLEMENTED /* involuntary context switches */
}

impl RUsage {
    pub fn new() -> Self {
        Self {
            ru_utime: TimeVal::new(),
            ru_stime: TimeVal::new(),
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        }
    }

    // pub fn add_utime(&mut self, usec: usize){
    //     self.ru_utime.add_usec(usec);
    // }

    // pub fn add_stime(&mut self, usec: usize){
    //     self.ru_stime.add_usec(usec);
    // }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}
