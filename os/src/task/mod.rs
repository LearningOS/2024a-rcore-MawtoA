//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
#[allow(rustdoc::private_intra_doc_links)]
mod task;

use crate::fs::{open_file, OpenFlags};
use crate::{config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::{is_mapped, MapPermission, VirtAddr, VirtPageNum}, timer::get_time_ms};
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
use task::SyscallInfo;
pub use task::{TaskControlBlock, TaskStatus};

pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    // drop file descriptors
    inner.fd_table.clear();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("ch6b_initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// 计算程序运行时长
pub fn calc_task_time() -> usize {
    let launch_time = current_task()
        .unwrap()
        .inner_exclusive_access()
        .launch_time;
    let current_time = get_time_ms();
    current_time - launch_time
}

/// 获得程序运行状态
pub fn task_status() -> TaskStatus {
    current_task().unwrap().inner_exclusive_access().task_status
}

/// 使特定 syscall 的计数加一
pub fn add_syscall_count(task_id: usize) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    for syscall in &mut inner.syscalls {
        if syscall.id == task_id || syscall.id == 0 {
            syscall.id = task_id;
            syscall.times += 1;
            return;
        }
    }
    inner.syscalls.push(SyscallInfo {
        id: task_id,
        times: 1,
    });
}

/// 统计所有 syscall 的调用次数
pub fn syscall_statistics(dst: &mut [u32; MAX_SYSCALL_NUM]) {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    for syscall in &inner.syscalls {
        if syscall.id == 0 {
            break;
        }
        dst[syscall.id] = syscall.times;
    }
}

/// 校验分配是否合理
fn is_mmap_valid(start: usize, end: usize, port: usize) -> bool {
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
fn is_munmap_valid(start: usize, end: usize) -> bool {
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
pub fn mmap(start: usize, end: usize, port: usize) -> isize {
    if !is_mmap_valid(start, end, port) {
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
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .memory_set
        .insert_framed_area(VirtAddr::from(start), VirtAddr::from(end), mp);
    0
}

/// 申请解分配内存
pub fn munmap(start: usize, end: usize) -> isize {
    if !is_munmap_valid(start, end) {
        return -1;
    }
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .memory_set
        .delete_framed_area(VirtAddr::from(start), VirtAddr::from(end));
    0
}
