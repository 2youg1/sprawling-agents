-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::change

规定 `kernel::change`（`crates/kernel/src/change.rs`）：一个文件变更的形状。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::change` 旁的测试守住。
-/

/-!
### 8-30 kernel::change（形状 2 value）

```rust
pub enum Lines { Counted { added: u32, removed: u32 }, Binary }
pub enum How   { Added, Modified, Deleted, Renamed { from: String } }
pub struct FileChange { pub path: String, pub how: How, pub lines: Lines }
```

**为什么在 kernel 而不在产地**：三处需要这个形状——`storage::changes` 从两棵树上读出它、
`wire::frames` 携它、客户端画它——而 `wire` 看不见 `storage`。定义两份再互相转换，
就是「一个文件变更是什么」有两个定义，而漂开的总是没人看的那个。这与 `Restoration` 当初落在这里
是同一条理由。

**`Lines` 是穷举枚而不是两个 `u32`**：二进制文件没有行数。把它拼成 `Counted { 0, 0 }`
与「碰过但没改」同形，而那是界面在报一个没人做过的测量——一条断言钉住两者序列化后不同形。

**`How::Renamed` 自带来处**：改名与「删一个加一个」是关于同两棵树的两个事实；
一个在判断 agent 是搬了代码还是重写了代码的人，需要这个差别。

**没有任何字段能装补丁文本**：补丁文本就是文件内容，而文件内容离开运行中的机器是
`secret::scan` 存在的理由。hunk 必须单独请求并同样受扫，所以它拼不进这个类型。
-/
