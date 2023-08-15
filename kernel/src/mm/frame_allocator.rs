//! We use FrameTracker to manage the lifecycle of physical page frames
//! (which cannot be derived as Clone/Copy traits), so all operations on
//! FrameTracker are accompanied by tracking its own reference count.
//!
//! The reference counter is located in the allocator StackFrameAllocator.
//! It is necessary to analyze the operations on the reference counter carefully.
//!
//! In the future, improvements could potentially be made using Arc provided by Rust
//! to manage the reference count.
//!
//! For those who have learned from rCore-tutorial, you may notice that rCore does
//! not introduce a reference counter in StackFrameAllocator.
//! In fact, we use the reference counter to implement the Copy-on-Write mechanism,
//! which is not implemented in rCore-tutorial.
//! However, it is not necessary to manually maintain the reference counter to
//! implement the Copy-on-Write mechanism. Many excellent teams have also implemented
//! the Copy-on-Write mechanism， and you can refer to their implementations.

use super::address::{PhysAddr, PhysPageNum};
use crate::consts::PHYS_END;
use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt::{self, Debug, Formatter};
use spin::Mutex;

/// Physical Page Frame, which represents a physical page in
/// RAM, identified by a physical page number.
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}
impl FrameTracker {
    /// Create a new FrameTracker from the given physical page number.
    /// This function will initialize the memory to 0.
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_arr = ppn.as_bytes_array();
        bytes_arr.into_iter().for_each(|b| *b = 0);
        frame_add_ref(ppn);
        Self { ppn }
    }
    pub fn from_ppn(ppn: PhysPageNum) -> Self {
        frame_add_ref(ppn);
        Self { ppn }
    }
}
impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}
impl Drop for FrameTracker {
    fn drop(&mut self) {
        dealloc_frame(self.ppn);
    }
}
impl Clone for FrameTracker {
    fn clone(&self) -> Self {
        FrameTracker::from_ppn(self.ppn)
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
    fn add_ref(&mut self, ppn: PhysPageNum);
    fn enquire_ref(&self, ppn: PhysPageNum) -> usize;
    fn usage(&self) -> (usize, usize, usize, usize);
}

pub struct StackFrameAllocator {
    base_num: usize,
    end: usize,
    /// Current is the next free frame that can be allocated.
    current: usize,
    /// The recycled physical page numbers are saved in a FIFO manner for future use.
    recycled: Vec<usize>,
    /// Reference counter for each physical page.
    refcounter: BTreeMap<usize, u8>,
}
impl StackFrameAllocator {
    /// - `l`: Free memory start page number
    /// - `r`: Free memory end page number
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        self.base_num = l.0;
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            base_num: 0,
            current: 0,
            end: 0,
            recycled: Vec::new(),
            refcounter: BTreeMap::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        // Check if there is any recycled physical page number in the stack `recycled`.
        // If there is, pop the top of the stack and return it.
        if let Some(ppn) = self.recycled.pop() {
            self.refcounter.insert(ppn, 0);
            Some(ppn.into())
        }
        // if is full, return None
        else if self.current == self.end {
            None
        }
        // else return the current frame and increase the current frame by 1.
        else {
            self.current += 1;
            self.refcounter.insert(self.current - 1, 0);
            Some((self.current - 1).into())
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        let option = self.refcounter.get_mut(&ppn);
        if option.is_none() {
            panic!(
                "[StackFrameAllocator::dealloc] Frame ppn={:#x} has no reference!",
                ppn
            );
        }

        let ref_times = self.refcounter.get_mut(&ppn).unwrap();
        assert!(
            *ref_times > 0,
            "[StackFrameAllocator::dealloc] Frame ppn={:#x} has no reference!",
            ppn
        );
        *ref_times -= 1;
        if *ref_times == 0 {
            self.refcounter.remove(&ppn);
            // Validate the physical page number. If the physical page number is greater than the
            // highest memory allocated or the physical page number exists in the freed stack,
            // panic.
            if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
                panic!(
                    "[StackFrameAllocator::dealloc] Frame ppn={:#x} has not been allocated!",
                    ppn
                );
            }
            // recycle the physical page number by pushing it into the stack `recycled`.
            self.recycled.push(ppn);
        }
    }
    fn usage(&self) -> (usize, usize, usize, usize) {
        (self.current, self.recycled.len(), self.end, self.base_num)
    }
    fn add_ref(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        let ref_times = self.refcounter.get_mut(&ppn).unwrap();
        *ref_times += 1;
    }
    fn enquire_ref(&self, ppn: PhysPageNum) -> usize {
        let ppn = ppn.0;
        let ref_times = self.refcounter.get(&ppn).unwrap();
        (*ref_times).clone() as usize
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// Physical Page Frame Manager Instance
    /// - Global variable responsible for managing memory space outside of the kernel space.
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

/// Initialize Physical Page Frame Manager
/// - Physical Page Frame Range
/// - Obtain the starting physical page number by rounding up the physical address of ekernel
/// - Obtain the ending physical page number by rounding down the physical address of PHYS_END
pub fn init() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.lock().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(PHYS_END).floor(),
    );
}
pub fn alloc_frame() -> Option<FrameTracker> {
    let ppn = FRAME_ALLOCATOR.lock().alloc()?;
    Some(FrameTracker::new(ppn))
}
pub fn dealloc_frame(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().dealloc(ppn);
}
pub fn frame_add_ref(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().add_ref(ppn)
}
pub fn enquire_refcount(ppn: PhysPageNum) -> usize {
    FRAME_ALLOCATOR.lock().enquire_ref(ppn)
}
