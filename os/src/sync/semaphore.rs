//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// sema id
    pub id: usize,
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub owner_queue: VecDeque<Option<Arc<TaskControlBlock>>>,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    fn dfs_dead(
        &self,
        task_id: usize,
        sema_vis: &mut Vec<usize>,
        task_vis: &mut Vec<usize>,
    ) -> bool {
        if sema_vis.contains(&self.id) {
            return false;
        }
        sema_vis.push(self.id);
        let self_inner = self.inner.const_access();
        for owner in &self_inner.owner_queue {
            if owner.is_none() {
                continue;
            }
            let owner = owner.as_ref().unwrap().inner_const_access();
            let id = owner.trap_cx_ppn.0;
            if task_vis.contains(&id) {
                continue;
            }
            if task_id == id {
                return true;
            }
            task_vis.push(id);
            for sema in &owner.sema_list {
                if sema.is_none() || sema.as_ref().unwrap().upgrade().is_none() {
                    continue;
                }
                let sema = sema.as_ref().unwrap().upgrade().unwrap();
                if sema_vis.contains(&sema.id) {
                    continue;
                }
                if sema.dfs_dead(task_id, sema_vis, task_vis) {
                    return true;
                }
            }
        }
        false
    }
    /// 拓扑排序判环
    ///
    /// 参考文章：[\[图论\]判环的几种方法](https://www.cnblogs.com/nannandbk/p/17739265.html)
    fn has_dead(&self, task_id: usize) -> bool {
        let mut sema_vis = Vec::new();
        let mut task_vis = Vec::new();
        self.dfs_dead(task_id, &mut sema_vis, &mut task_vis)
    }
    /// Create a new semaphore
    pub fn new(id: usize, res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            id,
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    owner_queue: VecDeque::new(),
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        let id = current_task()
            .unwrap()
            .inner_exclusive_access()
            .trap_cx_ppn
            .0;
        inner
            .owner_queue
            .iter_mut()
            .filter(|owner| {
                owner.is_some()
                    && owner
                        .as_ref()
                        .unwrap()
                        .inner_exclusive_access()
                        .trap_cx_ppn
                        .0
                        == id
            })
            .for_each(|owner| {
                owner
                    .as_ref()
                    .unwrap()
                    .inner_exclusive_access()
                    .sema_list
                    .iter_mut()
                    .filter(|sema| {
                        sema.is_some()
                            && sema.as_ref().unwrap().upgrade().is_some()
                            && sema.as_ref().unwrap().upgrade().unwrap().id == self.id
                    })
                    .for_each(|sema| {
                        sema.take();
                    });
                owner.take();
            });
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                inner.owner_queue.push_back(Some(Arc::clone(&task)));
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(self: Arc<Self>) -> isize {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            drop(inner);
            if self.has_dead(current_task().unwrap().inner_const_access().trap_cx_ppn.0) {
                return -0xdead;
            }
            let mut inner = self.inner.exclusive_access();
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            let task = current_task().unwrap();
            task.inner_exclusive_access()
                .sema_list
                .push(Some(Arc::downgrade(&self)));
            inner.owner_queue.push_back(Some(Arc::clone(&task)));
        }
        0
    }
}
