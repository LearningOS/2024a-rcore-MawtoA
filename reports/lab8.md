## Chapter8 实验报告

### 功能实现

为了检测死锁，本次实验通过维护资源图在线程申请上锁/减少信号量失败时检查等待队列是否有需要该线程释放资源才能释放锁的线程，如果检测到则判定死锁发生。为了解决搜索资源成环过程中的借用所有权问题，`UPSafeCell` 添加了取出不可变借用的 `const_access` 方法。

### 简答部分

1. 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

    * 需要回收的资源可以包括任务队列（所有存活线程的 `TaskControlBlock`）、进程占用的内存（`ProcessControlBlockInner::memory_set`）和使用的文件（`ProcessControlBlockInner::fd_table`）和进程使用的锁 (`ProcessControlBlockInner::mutex_list`) 等等。
    * 线程的 `TaskControlBlock` 可能在调用系统函数时、申请并发资源时（用于维护等待队列）、进程维护线程队列时被引用。前两者是在线程活跃时出现的，一般无需回收；后者则依维护时的具体需要而定，比如线程已关闭时`TaskControlBlock` 就有必要得到清理。

2. 对比以下两种 `Mutex.unlock` 的实现，二者有什么区别？这些区别可能会导致什么问题？

        impl Mutex for Mutex1 {
            fn unlock(&self) {
                let mut mutex_inner = self.inner.exclusive_access();
                assert!(mutex_inner.locked);
                mutex_inner.locked = false;
                if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
                    add_task(waking_task);
                }
            }
        }

        impl Mutex for Mutex2 {
            fn unlock(&self) {
                let mut mutex_inner = self.inner.exclusive_access();
                assert!(mutex_inner.locked);
                if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
                    add_task(waking_task);
                } else {
                    mutex_inner.locked = false;
                }
            }
        }

    二者的区别主要在于，`Mutex1` 先解锁再调取等待队列，`Mutex2` 则在确定等待队列清空后解锁。两者的区别可能使：
    * 如果 unlock 函数不是原子的（本身执行时其它任务不能停止），那么 `Mutex1` 在解锁到调取等待队列的步骤间可能会被其它进程抢占锁，出现数据竞争。
    * 依据 add_task 的设计是否考虑设置 `mutex_inner.locked` 的值，两种方案可能分别出现 `mutex_inner.locked` 值的相关错误。

### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我没有就实验相关内容与他人做过交流。

2. 我参考了博文 [\[图论\]判环的几种方法](https://www.cnblogs.com/nannandbk/p/17739265.html)，并在代码中对应的位置以注释形式记录了具体的参考来源及内容。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。