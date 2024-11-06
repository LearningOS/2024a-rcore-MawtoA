//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_SYSCALL_NUM, PAGE_SIZE};
use crate::loader::{get_app_data, get_num_app};
use crate::mm::{is_mapped, MapPermission, VirtAddr, VirtPageNum};
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::SyscallInfo;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        if next_task.task_launch_time == 0 {
            next_task.task_launch_time = get_time_ms();
        }
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            if inner.tasks[next].task_launch_time == 0 {
                inner.tasks[next].task_launch_time = get_time_ms();
            }
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// 计算程序运行时长
    fn calc_task_time(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let task = &inner.tasks[inner.current_task];

        let launch_time = task.task_launch_time;
        let current_time = get_time_ms();

        current_time - launch_time
    }

    /// 获得程序运行状态
    fn task_status(&self) -> TaskStatus {
        let inner = self.inner.exclusive_access();
        let task = &inner.tasks[inner.current_task];

        task.task_status
    }

    /// 使特定 syscall 的计数加一
    fn add_task(&self, task_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        let task = &mut inner.tasks[cur];

        for syscall in &mut task.task_syscall_times {
            if syscall.id == task_id || syscall.id == 0 {
                syscall.id = task_id;
                syscall.times += 1;
                return;
            }
        }

        task.task_syscall_times.push(SyscallInfo {
            id: task_id,
            times: 1,
        });
    }

    /// 统计所有 syscall 的调用次数
    fn syscall_statistics(&self, dst: &mut [u32; MAX_SYSCALL_NUM]) {
        let inner = self.inner.exclusive_access();
        let task = &inner.tasks[inner.current_task];

        for syscall in &task.task_syscall_times {
            if syscall.id == 0 {
                break;
            }
            dst[syscall.id] = syscall.times;
        }
    }

    /// 校验分配是否合理
    fn is_mmap_valid(&self, start: usize, end: usize, port: usize) -> bool {
        if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
            return false;
        }
        let mut start = start;
        while start < end {
            if is_mapped(
                current_user_token(),
                VirtPageNum::from(VirtAddr::from(start)),
            ) {
                return false;
            }
            start += PAGE_SIZE;
        }
        true
    }

    /// 校验解分配是否合理
    fn is_munmap_valid(&self, start: usize, end: usize) -> bool {
        if start % PAGE_SIZE != 0 {
            return false;
        }
        let mut start = start;
        while start < end {
            if !is_mapped(
                current_user_token(),
                VirtPageNum::from(VirtAddr::from(start)),
            ) {
                return false;
            }
            start += PAGE_SIZE;
        }
        true
    }

    /// 申请内存
    fn mmap(&self, start: usize, end: usize, port: usize) -> isize {
        if !self.is_mmap_valid(start, end, port) {
            return -1;
        }
        let mut mp = MapPermission::U;
        if port & (1 << 0) != 0 {
            mp |= MapPermission::R;
        }
        if port & (1 << 1) != 0 {
            mp |= MapPermission::W;
        }
        if port & (1 << 2) != 0 {
            mp |= MapPermission::X;
        }
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].memory_set.insert_framed_area(
            VirtAddr::from(start),
            VirtAddr::from(end),
            mp,
        );
        0
    }

    /// 申请解分配内存
    fn munmap(&self, start: usize, end: usize) -> isize {
        if !self.is_munmap_valid(start, end) {
            return -1;
        }
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur]
            .memory_set
            .delete_framed_area(VirtAddr::from(start), VirtAddr::from(end));
        0
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// 计算程序运行时长
pub fn calc_task_time() -> usize {
    TASK_MANAGER.calc_task_time()
}

/// 获得程序运行状态
pub fn task_status() -> TaskStatus {
    TASK_MANAGER.task_status()
}

/// 使特定 syscall 的计数加一
pub fn add_syscall_count(task_id: usize) {
    TASK_MANAGER.add_task(task_id);
}

/// 统计所有 syscall 的调用次数
pub fn syscall_statistics(dst: &mut [u32; MAX_SYSCALL_NUM]) {
    TASK_MANAGER.syscall_statistics(dst);
}

/// 申请内存
pub fn mmap(start: usize, end: usize, port: usize) -> isize {
    TASK_MANAGER.mmap(start, end, port)
}

/// 申请解分配内存
pub fn munmap(start: usize, end: usize) -> isize {
    TASK_MANAGER.munmap(start, end)
}
