## Chapter4 实验报告

### 功能实现

本次实验中，我利用 `PhysAddr::get_mut` 方法，模仿 `translated_byte_buffer` 设计了 `translated_mut` 函数以供 `syscall` 模块通过虚拟地址空间访问变量。

我还利用 `insert_framed_area` 和自行设计的 `delete_framed_area` 方法实现了 `mmap` 和 `munmap` 方法。

### 简答部分

1. 请列举 SV39 页表页表项的组成，描述其中的标志位有何作用？

    从低到高依次是 V, R, W, X, U, G, A, D 八位。它们的含义是：

    * V - Valid: 页表项是否合法。
    * R - Read: 页表项对应的虚拟页面是否可读。
    * W - Write: 页表项对应的虚拟页面是否可写。
    * X - eXecute: 页表项对应的虚拟页面是否可执行。
    * U - User: 该页表项在 CPU 处于 U 态时是否可被访问。
    * G - Global: 页表项对应的映射是否是全局的。在当前实现中可以忽略。
    * A - Accessed: 页表项对应的虚拟页面在该位上一次清零后是否被访问过。
    * D - Dirty: 页表项对应的虚拟页面在该位上一次清零后是否被修改过。

2. **缺页**

    缺页指的是进程访问页面时页面不在页表中或在页表中无效的现象，此时 MMU 将会返回一个中断， 告知 os 进程内存访问出了问题。os 选择填补页表并重新执行异常指令或者杀死进程。

    * 请问哪些异常可能是缺页导致的？

        1. 物理内存中没有对应的页帧；
        2. 物理内存中有对应的页帧，但并未在页表中建立映射；
        3. 物理内存中有对应的页帧，但当前进程的访问无效。

    * 发生缺页时，描述相关重要寄存器的值，上次实验描述过的可以简略。

        1. `scause` - 储存 Trap 的具体信息
        2. `sepc` - 储存 Trap 发生时会将当前指令的下一条指令地址
        3. `sscratch` - 指向 Hart 相关的 S 态上下文的指针
        4. `sstatus` - 储存处理器当前状态
        5. `stval` - 储存与 Trap 相关的信息
        6. `stvec` - 储存处理 Trap 的指令入口地址

    缺页有两个常见的原因，其一是 Lazy 策略，也就是直到内存页面被访问才实际进行页表操作。 比如，一个程序被执行时，进程的代码段理论上需要从磁盘加载到内存。但是 os 并不会马上这样做， 而是会保存 .text 段在磁盘的位置信息，在这些代码第一次被执行时才完成从磁盘的加载操作。

    * 这样做有哪些好处？
    
        该策略可以避免程序使用中多余的加载（比如申请但未被使用的内存），提升程序性能。

    其实，我们的 mmap 也可以采取 Lazy 策略，比如：一个用户进程先后申请了 10G 的内存空间， 然后用了其中 1M 就直接退出了。按照现在的做法，我们显然亏大了，进行了很多没有意义的页表操作。

    * 处理 10G 连续的内存页面，对应的 SV39 页表大致占用多少内存 (估算数量级即可)？

        S / 512 = 10GB / 512 = 20MB，大致占用 20MB 内存。

    * 请简单思考如何才能实现 Lazy 策略，缺页时又如何处理？描述合理即可，不需要考虑实现。

        分配内存时暂时不生成实际的物理页帧，等到访问对应页面时触发相应的缺页异常，并由`trap handler`处理，在处理时再将内存分配。

    缺页的另一个常见原因是 swap 策略，也就是内存页面可能被换到磁盘上了，导致对应页面失效。

    * 此时页面失效如何表现在页表项(PTE)上？

        PTE_V 位为 0。

3. 双页表与单页表

    为了防范侧信道攻击，我们的 os 使用了双页表。但是传统的设计一直是单页表的，也就是说， 用户线程和对应的内核线程共用同一张页表，只不过内核对应的地址只允许在内核态访问。 (备注：这里的单/双的说法仅为自创的通俗说法，并无这个名词概念，详情见 KPTI )

    * 在单页表情况下，如何更换页表？

        仍然可以通过更换当前使用的 `memory_set` 地址。

    * 单页表情况下，如何控制用户态无法访问内核页面？（tips:看看上一题最后一问）

        将内核页面的 PTE_U 位设置为 0。

    * 单页表有何优势？（回答合理即可）

        可以省去在内核和用户态之间转换时更换页表的步骤。

    * 双页表实现下，何时需要更换页表？假设你写一个单页表操作系统，你会选择何时更换页表（回答合理即可）？

        单页表操作系统在更换任务时更换页表即可，而双页表下更换任务、切换特权级时都需要更换页表。

### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我没有就实验相关内容与他人做过交流。

2. 我参考了[《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)，并在代码中对应的位置以注释形式记录了具体的参考来源及内容。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。