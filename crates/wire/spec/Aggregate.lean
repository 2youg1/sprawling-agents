-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::aggregate

规定 `aggregate`（`crates/wire/src/` 下同名的文件）。从一个界面看几座城，只有查询与事件。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `wire::aggregate` 旁的测试守住。
-/

/-!
### 8-5 wire::aggregate（形状 1 判定 ＋ 形状 2 值类型）

```rust
pub struct CityLabel(String);                     // parse 拒空与控制字符
pub struct Upstream { label, address: String, token_digest: Option<B3Hash> }
pub struct Sighting { city: CityLabel, event: EventRecord }
pub struct Forwarded { address: String, query: Query }   // 无 Command 字段
pub struct Aggregate { /* BTreeMap<CityLabel, Upstream> */ }
impl Aggregate {
    pub fn attach(&mut self, Upstream);  pub fn detach(&mut self, &CityLabel) -> bool;
    pub fn cities(&self) -> impl Iterator<Item = &Upstream>;
    pub fn ask(&self, &CityLabel, Query) -> Result<Forwarded, AxError>;  // 唯一发送面
    pub fn merge(Vec<(CityLabel, Vec<EventRecord>)>) -> Vec<Sighting>;
}
```

**硬约束的存放形式**：它只有一句「聚合层只转发 Query 与 Event，恒不转发 Command」。它在这里**不是一个判断**，是一个缺席：`ask` 接 `Query`，而没有第二个发送方法。连 `Forwarded` 也不带 Command 字段，使下游无处升格。理由：一个可代发命令的聚合层是**不在任何一本账上的跨城权威**——目标城无法把命令与发起人对应。

**合流序为什么不能用 seq**：两座 City 各有各的 Ledger，各自从 1 编号。故排序键取 `(t, city, seq)`：时间先行，label 入键而非做最后的破平——因为两城同一毫秒是常态，而合流视图必须两次一样。

**本模块不含传输**：需要确定性与可测性的是合流序与转发面，两者都不需要 socket。实际连接属装配层（界面接入时）。
-/
