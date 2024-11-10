//! Mutex (spin-like and blocking(sleep))

use core::cell::RefMut;

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(self: Arc<Self>) -> isize;
    /// Unlock the mutex
    fn unlock(&self, id: usize);
    /// set id
    fn set_id(&self, id: usize);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(self: Arc<Self>) -> isize {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return 0;
            }
        }
    }

    fn unlock(&self, _id: usize) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }

    fn set_id(&self, _id: usize) {}
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    id: UPSafeCell<usize>,
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    owner: Arc<TaskControlBlock>,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new(owner: Arc<TaskControlBlock>) -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            id: unsafe { UPSafeCell::new(0) },
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    owner,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
    /// 拓扑排序判环
    ///
    /// 参考文章：[\[图论\]判环的几种方法](https://www.cnblogs.com/nannandbk/p/17739265.html)
    pub fn has_circle(
        &self,
        mutex_inner: &RefMut<'_, MutexBlockingInner>,
        task_id: usize,
        mutex_vis: &mut Vec<usize>,
        task_vis: &mut Vec<usize>,
    ) -> bool {
        let id = *self.id.exclusive_access();
        if mutex_vis.contains(&id) {
            return false;
        }
        mutex_vis.push(id);
        for task in &mutex_inner.wait_queue {
            let id = task.inner_exclusive_access().trap_cx_ppn.0;
            if task_vis.contains(&id) {
                continue;
            }
            if id == task_id {
                return true;
            }
            task_vis.push(id);
            for mutex in &task.inner_exclusive_access().block_mutex_list {
                if mutex.is_some() {
                    return true;
                }
                let mutex = mutex.as_ref().unwrap();
                if mutex_vis.contains(&*mutex.id.exclusive_access()) {
                    continue;
                }
                if mutex.has_circle(
                    &mutex.inner.exclusive_access(),
                    task_id,
                    mutex_vis,
                    task_vis,
                ) {
                    return true;
                }
            }
        }
        false
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(self: Arc<Self>) -> isize {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        let task = current_task().unwrap();
        if mutex_inner.locked {
            let id = task.inner_exclusive_access().trap_cx_ppn.0;
            let mut mutex_vis = Vec::new();
            let mut task_vis = Vec::new();
            if mutex_inner.owner.inner_exclusive_access().trap_cx_ppn.0 == id
                || self.has_circle(&mutex_inner, id, &mut mutex_vis, &mut task_vis)
            {
                drop(mutex_inner);
                return -0xdead;
            }
            task.inner_exclusive_access()
                .block_mutex_list
                .push(Some(Arc::clone(&self)));
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
            0
        } else {
            task.inner_exclusive_access()
                .block_mutex_list
                .push(Some(Arc::clone(&self)));
            mutex_inner.owner = Arc::clone(&task);
            mutex_inner.locked = true;
            0
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self, id: usize) {
        trace!("kernel: MutexBlocking::unlock");
        let task = current_task().unwrap();
        task.inner_exclusive_access()
            .block_mutex_list
            .iter_mut()
            .filter(|mutex| mutex.is_some() && *mutex.as_ref().unwrap().id.exclusive_access() == id)
            .for_each(|mutex| {
                mutex.take();
            });
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            mutex_inner.owner = Arc::clone(&waking_task);
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }

    fn set_id(&self, id: usize) {
        *self.id.exclusive_access() = id;
    }
}
