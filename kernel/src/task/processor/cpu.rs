use alloc::{collections::VecDeque, sync::Arc};

use crate::task::TaskControlBlock;

use super::Processor;

pub struct Cpu {
    task_rq: VecDeque<Arc<TaskControlBlock>>,
    processor: Processor,
}
