-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::room

规定 `room`（`crates/city/src/` 下同名的文件）。一个具名会话在哪个房间里干活，新房间怎样出生。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::room` 旁的测试守住。
-/

/-!
### 8-13 city::room（形状 1 判定 ＋ 一个实例化动作）

```rust
pub fn open(city_root: &Path, building: &Address, name: &SessionName) -> Result<Address, AxError>;
pub fn claim(city_root: &Path, room: &Address) -> Result<(), AxError>;   // 门面上叫 `city::claim_room`；城替派活新建的房间：建出并封上；已存在的目录不动（§8-21）
// crate 面：`pub use room::open as open_room;`——调用方读到的是 `city::open_room`，
// 因为裸的 `city::open` 在装配层里说不出开的是什么。
```

与 `city::building` 同形：一个判定加一个落盘动作，而不是一个长住的值。

- **同名加后缀，不复用**：`refactor`、`refactor-2`。复用会把两次只是共用一个词的会话放进同一套文件，而那正是本模块要消掉的缺陷。「继续上一次会话」是向它已有的房间地址派活，由界面给出选项，而不是靠拼写撞对。
- **`create_dir` 而非先问后建**：问与答是一个操作，于是同一毫秒里的两次派活不会被同时告知「这个名字空着」。
- **999 个后缀封顶**：一个写错的循环应该停下来，而不是把盘写满。
-/
