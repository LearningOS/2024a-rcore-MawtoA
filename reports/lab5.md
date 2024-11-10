## Chapter5 实验报告

### 功能实现

本次实验中，我通过观察并组合 fork 和 exec 的部分功能实现了“新地址空间下执行进程”的 spawn 功能；同时通过修改 `TaskControlBlock` 和 `TaskControlBlockInner` 并补充了辅助函数，完成了 stride 调度。

### 简答部分 - stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

* 实际情况是轮到 p1 执行吗？为什么？

    不一定，由于 8bit 无符号整型可表示的范围在 0~255 之间，一旦 `BIG_STRIDE >= 60`，那么 p2.stride 会由于整型上溢变为一个小于等于 255 的值，此时算法很可能会认为 p2.stride < p1.stride 从而继续先执行 p2。

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

* 为什么？尝试简单说明（不要求严格证明）。

    由于进程优先级 >= 2，每次调度后进程优先级增量都不会超过“BigStride / 2”。而由于 stride 调度算法总是先执行 stride 值小的进程，在 STRIDE_MIN 超过 STRIDE_MAX 之前两者之差不会再度增大，而 STRIDE_MIN 也不可能一次就增加到大于 STRIDE_MAX + BigStride / 2，因此上述式子成立。

* 已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 `partial_cmp` 函数，假设两个 Stride 永远不会相等。

        use core::cmp::Ordering;

        struct Stride(u64);

        impl PartialOrd for Stride {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                let (smaller, bigger) = if self.0 < other.0 {
                    (self.0, other.0)
                } else {
                    (other.0, self.0)
                };
                let diff = bigger - smaller;
                let max_diff = BIG_STRIDE / 2;
                if self.0 < other.0 && diff <= max_diff || self.0 > other.0 && diff > max_diff {
                    Sone(Ordering::Less)
                } else {
                    Sone(Ordering::Greater)
                }
            }
        }

        impl PartialEq for Stride {
            fn eq(&self, other: &Self) -> bool {
                false
            }
        }

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: `(125 < 255) == false`, `(129 < 255) == true`.

### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我没有就实验相关内容与他人做过交流。

2. 我参考了[《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)，并在代码中对应的位置以注释形式记录了具体的参考来源及内容。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。