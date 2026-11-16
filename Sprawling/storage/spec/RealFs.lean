-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::real_fs

规定 `real_fs`（`crates/storage/src/` 下同名的文件）。Vfs 的生产适配器：std::fs，持住正在追写的那个句柄。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-16 storage::real_fs（形状 4 适配器）

```rust
pub(crate) struct RealFs { open: Option<HeldWriter> }   // std::fs 直译，零策略；句柄记着自己是追写还是按位置写
impl RealFs { pub(crate) fn new() -> RealFs; }
impl Vfs for RealFs { … }
/// 段文件的字节怎样到介质：每一臂在屏障处照样 `sync_data`，所以 `append_all` 答 `Ok` 的持久语义各臂相同，臂只改代价（storage D24）。
pub(crate) enum SegmentDurability { SyncData, WriteThrough }
/// 每个平台一个常量；选臂就是改这一个值。今天三个平台都是 `SyncData`。
pub(crate) const SEGMENT_DURABILITY: SegmentDurability;
```

- **写直达只在有它的平台上能选**：Windows 经 `std::os::windows::fs::OpenOptionsExt::custom_flags` 加 `FILE_FLAG_WRITE_THROUGH`（`0x8000_0000`），Linux 经 `std::os::unix::fs::OpenOptionsExt::custom_flags` 加 `O_DSYNC`（`0o10000`，只在 `asm-generic` 标志表给出这个值的 x86_64、x86、aarch64、arm、riscv64 上；本 crate 不依赖 `libc`，所以这个值写在这里）。macOS 与其余平台没有这一臂：那里的常量若写成 `WriteThrough`，编译期断言拒绝它，而不是静默退回 `SyncData`。
- **写直达仍发屏障**：Linux 上 `O_DSYNC` 按 POSIX 已含 `fdatasync` 的保证，Windows 上写直达是否连文件长度一起落盘，微软的文档没有说清（推断，未在掉电下验证），所以两处都照样 `sync_data`；臂的问题只剩代价，留给发布构建上的读数。

- **唯一的状态是那个句柄**：写与 sync 走同一个句柄，所以被做持久的就是刚写的那些字节，也省掉每条记录两次重开（§7）。`append` 用追写模式的句柄，`write_at` 用按位置写的句柄（先 `seek` 再写，storage D31）；所持句柄的模式不同就换一个，`sync_data` 用手上那一个。
- **句柄命名的是文件不是路径**：`rename`／`remove_file`／`truncate` 前必须 `release`。两条断言随这个模块走，它们问的是「之后字节落在哪个文件里」。
- **`rename` 是原子替换，在 Windows 上也是**：Windows 拒绝把文件改名到一个只读文件上（拒绝访问），而 `Vfs::rename` 的契约是替换。所以在 Windows 上，改名因无权被拒且目标是只读文件时，先清掉目标的只读位再改名一次；别的原因的拒绝原样返回。成功路径不多花一次系统调用。两步之间崩溃，目标内容不变、只丢只读位；暂存文件已带原权限（8-25），重做一次落盘即复原。被否：在 `bundle::landing` 里先清目标的位——那要给端口加一个方法，而规则属于「替换」本身，所有经 `rename` 的替换都该守它。
-/
