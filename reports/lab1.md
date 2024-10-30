## Chapter3 实验报告

### 功能实现

本次实验中，我扩展了 `TaskControlBlock` ，添加表示任务首次运行时间和系统调用情况的属性，并在 `task` 模块中中添加了记录开始时运行时间的指令，编写了更新和获得 `task_info` 三个成员所需的函数并添加到系统指令实现中。通过以上修改，本系统现在可以查询当前正在执行的任务信息，包括任务状态、使用的系统调用及调用次数、系统调用距任务第一次被调度时刻的时长。

### 简答部分

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ），描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

    使用 RustSBI version 0.3.0-alpha.2 (QEMU Version 0.2.0-alpha.2) 对上述测例进行测试。

    1. 运行 `ch2b_bad_address` 时，系统中止了该程序运行并向控制台打印了如下错误信息：

            [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003c8, kernel killed it.

        分析源代码可以确定，用户程序尝试向地址为 `0x0` 的内存写入数据，触发了储存错误异常并 Trap 使程序提前退出（因此源代码中的 panic 代码没有执行）。

    2. 运行 `ch2b_bad_instructions` 时，系统中止了该程序运行并向控制台打印了如下错误信息：

            [kernel] IllegalInstruction in application, kernel killed it.

        分析源代码可以确定，用户程序尝试在 U 态下执行 S 态特权指令 `sret`，触发了非法指令异常并 Trap 使程序提前退出。

    3. 运行 `ch2b_bad_register` 时系统的表现与运行 `ch2b_bad_instructions` 时非常相似，不过触发非法指令异常的原因是用户程序尝试在 U 态下通过 `csrr` 指令读取 S 态寄存器 `sstatus` 中的值。

2. 深入理解 trap.S 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

    1. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。

    2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

            ld t0, 32*8(sp)
            ld t1, 33*8(sp)
            ld t2, 2*8(sp)
            csrw sstatus, t0
            csrw sepc, t1
            csrw sscratch, t2

    3. L50-L56：为何跳过了 `x2` 和 `x4`？

            ld x1, 1*8(sp)
            ld x3, 3*8(sp)
            .set n, 5
            .rept 27
            LOAD_GP %n
            .set n, n+1
            .endr

    4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

            csrrw sp, sscratch, sp

    5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

    6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

            csrrw sp, sscratch, sp

    7. 从 U 态进入 S 态是哪一条指令发生的？

### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我没有就实验相关内容与他人做过交流。

2. 我参考了[《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)，并在代码中对应的位置以注释形式记录了具体的参考来源及内容。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。