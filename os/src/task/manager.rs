//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.round_robin_schedule()
        self.stride_schedule()
    }

    /// todo doc
    #[allow(unused)]
    fn round_robin_schedule(&mut self) -> Option<Arc<TaskControlBlock>> {
        Some(self.ready_queue.remove(0))
    }

    /// todo doc
    #[allow(unused)]
    fn stride_schedule(&mut self) -> Option<Arc<TaskControlBlock>> {
        let (i, _) = self
            .ready_queue
            .iter()
            .enumerate()
            // .inspect(|&(i, tcb)| {
            //     println!(
            //         "task {}, stride: {}",
            //         i,
            //         tcb.inner_exclusive_access().stride
            //     )
            // })
            .min_by_key(|&(_, tcb)| tcb.inner_exclusive_access().stride)?;
        let tcb = self.ready_queue.remove(i);
        // println!("choose {}", i);
        tcb.update_stride();
        Some(tcb)
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
