## Chapter6/7 实验报告

这是一篇占位文件。剩下的部分将在编程作业完成后补充。

### 功能实现

### 简答部分

1. 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

    起到的是根目录中的作用。ROOT_INODE 损坏，根据损坏位置不同，可能出现：
    * 无法找到应该存在的文件。
    * 原本存在的文本文件出现错误，二进制文件无法运行。
    * 文件管理系统完全崩溃，无法使用。

2. 举出使用 pipe 的一个实际应用的例子。

    使用 `apt list` 查看 Ubuntu 下安装的 apt 包时经常会由于自动安装的依赖包数量过多无法很好的梳理文件情况。因此我一般利用管道将结果传递给 grep 过滤，通过命令：

        apt list --installed | grep -v automatic

### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我在实验的一个中间版本的 get_inode_id 实现时受到了微信群友 hatachi 和 Dynamic_Pigeon 讨论的启发，还在代码中对应的位置以注释形式记录了具体的内容。

2. 我参考了[《rCore-Tutorial-Book 第三版》](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)，并在代码中对应的位置以注释形式记录了具体的参考来源及内容。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。