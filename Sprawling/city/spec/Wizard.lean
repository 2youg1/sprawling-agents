-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::wizard

规定 `wizard`（`crates/city/src/` 下同名的文件）。建城向导：一个目录能形成什么样的城，一句指令的城由什么构成。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::wizard` 旁的测试守住。
-/

/-!
### 8-10 city::wizard（形状 1 判定＋形状 2 值类型；含 survey）

```rust
pub enum Standing {
    Empty,
    Work { adoptable: Vec<Address>, loose: usize },
    AlreadyACity,
}
pub fn survey(entries: &[(String, bool)], has_history: bool) -> Standing;
```

- **「目录非空」不是一个人能据以行动的答案**：三个臂各自说清接下来会发生什么，因为指着自己干了一年的文件夹的那个人要知道**会往他的工作旁边放什么、不会动什么**。
- **收列表而不是收路径**：本模块的全部纪律就是「决定是值，落盘是二进制的活」；判断一座城形成在这里会做什么，不该需要一块磁盘。
- **地址语法拼不出的名字算 loose**：一栋 dispatch 不到的楼不是楼。点目录、含 `:` 或 `\` 的名字都归此类；**带空格的名字语法收得下，故照样可作楼**。
- **排序后再答**：两台机器读同一个目录必须得出同一个答案。

```rust
pub struct CityPlan { /* dirs、first 私有 */ }
impl CityPlan {
    pub fn new(first: Option<(&str, &str)>) -> Result<CityPlan, AxError>;   // 一句指令的城
    pub fn dirs(&self) -> &[Address];
    pub fn first(&self) -> Option<&(Address, BuildingTemplate)>;
}
```

- **是判定，不是动作**：新城由什么构成，在这里以值给出；建目录与落事件归 bin。这个切分使「一句指令建一座城」可以在不建城的条件下被断言。
- **没有 `city` 里的 office 模块**：三层配置（City／Building／Resident）已是完整的梯子，OFFICE.md 没有任何一条自有规则，第四层只会成为「同一个设置在哪儿写」的第二个答案。
-/
