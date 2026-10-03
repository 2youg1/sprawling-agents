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
pub(crate) struct RealFs { open: Option<OpenAppend> }   // std::fs 直译，零策略
impl RealFs { pub(crate) fn new() -> RealFs; }
impl Vfs for RealFs { … }
```

- **唯一的状态是那个句柄**：追写与 sync 走同一个句柄，所以被做持久的就是刚写的那些字节，也省掉每条记录两次重开（§7）。
- **句柄命名的是文件不是路径**：`rename`／`remove_file`／`truncate` 前必须 `release`。两条断言随这个模块走，它们问的是「之后字节落在哪个文件里」。
- **`rename` 是原子替换，在 Windows 上也是**：Windows 拒绝把文件改名到一个只读文件上（拒绝访问），而 `Vfs::rename` 的契约是替换。所以在 Windows 上，改名因无权被拒且目标是只读文件时，先清掉目标的只读位再改名一次；别的原因的拒绝原样返回。成功路径不多花一次系统调用。两步之间崩溃，目标内容不变、只丢只读位；暂存文件已带原权限（8-25），重做一次落盘即复原。被否：在 `bundle::landing` 里先清目标的位——那要给端口加一个方法，而规则属于「替换」本身，所有经 `rename` 的替换都该守它。
-/
