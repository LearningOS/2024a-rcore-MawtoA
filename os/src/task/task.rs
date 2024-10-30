//! Types related to task management

use super::TaskContext;
use crate::config::MAX_SYSCALLINFO_NUM;

/// 系统调用记录。
///
/// 使用结构体短数组的想法和结构体成员的设计受到
/// [《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)
/// 的启发。
#[derive(Copy, Clone)]
pub struct SyscallInfo {
    /// 调用的 syscall id
    pub id: usize,
    /// 总调用次数
    pub times: u32,
}

/// The task control block (TCB) of a task.
///
/// 对 `TaskControlBlock` 进行扩展的设计受到
/// [《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)
/// 的启发。
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// 任务首次运行的时间
    pub task_launch_time: usize,
    /// syscall 调用计数
    pub task_syscall_times: [SyscallInfo; MAX_SYSCALLINFO_NUM],
}

impl TaskControlBlock {
    /// 初始化 TaskControlBlock
    pub fn init() -> Self {
        Self {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_launch_time: 0,
            task_syscall_times: [SyscallInfo { id: 0, times: 0 }; MAX_SYSCALLINFO_NUM],
        }
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
