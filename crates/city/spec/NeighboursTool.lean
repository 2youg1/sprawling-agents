-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::neighbours_tool

规定 `neighbours_tool`（`crates/city/src/` 下同名的文件）。名册给模型的那一面。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::neighbours_tool` 旁的测试守住。
-/

/-!
### 8-15b city::neighbours_tool（形状 4 适配器）

```rust
pub struct NeighboursTool { /* meta＋Neighbourhood —— 私有 */ }
impl NeighboursTool { pub fn new(neighbourhood: Neighbourhood) -> Result<NeighboursTool, AxError>; }
impl Tool for NeighboursTool { /* name=neighbours、effect=Read、cost=Free、render=Generic、temporal=Timeless */ }
// args：{scope}，`building`（缺省）｜`city`；结果为 {text}，一行一个地址，BTreeMap 序
```

- **`scope` 的两个取值取自配置梯子已有的层名**（`Layer::{City, Building}`），不另造一套远近词。
- **答案是按序渲染的文本而非 JSON 数组**：与 `status` 同一条已被红测试抓出的理由——`serde_json::Map` 对键排序，没有读者可依赖的序；序是模型读到的东西的属性，故落在模型读到的地方。
- **表头把「没列出的名字没有读者」写在第一行**：这是本工具存在的那个缺陷的正面表述，放在模型最先读到的位置。
- **随 Run 冻结，与 catalog 同理**：名册是派活那一刻扫到的地址与自述。别的 lane 上的 run 可能在这一次 drive 之内开出新房间，这份名册要到下一次派活才看得见它；本 Run 发出的 signal 在 drive 结束后才投递。`Temporal::Timeless` 说的是这份冻结的名册在回合之间不变，而不是城在回合之间不变。
-/
