在这一步时，会遇到一个 error 
![img](./assets/elf_image_too_big_error.png)
这时我们就需要重新分配 s3 中 16M flash 的分区了。

通过 partitions.csv 可以手动来分配 16M flash 的分区划分。给 app partition 更大的空间。
在修改了分区划分之后，建议先执行 `espflash erase-flash` 来清除 s3 的 flash，以免遇到奇怪的bug。