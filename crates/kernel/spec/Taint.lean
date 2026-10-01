-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::taint

规定 `kernel::taint`（`crates/kernel/src/taint.rs`）：外来内容的来源标记。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-10 kernel::taint

```rust
pub struct TaintSource(String);            // 非空来源标签（如 "web:example.com"）；文法 P1 随 Endpoint 收紧
pub struct TaintSet(BTreeSet<TaintSource>); // 空集＝内生数据；并集半格
impl TaintSet { pub fn empty() -> Self;  pub fn union(&self, other: &TaintSet) -> TaintSet;
                pub fn is_empty(&self) -> bool;  pub fn contains(&self, s: &TaintSource) -> bool; }
impl Display for TaintSet;                  // 来源标签按集合次序以 ", " 连接

pub struct Tainted<T> { /* value, taint —— 字段私有 */ }
impl<T> Tainted<T> {
    /// Sole entrance for external content. Custody
    /// composition (secret scan before CAS) is the effect layer's wiring
    /// at this call site; the type itself stays pure.
    pub fn new(value: T, source: TaintSource) -> Self;
    pub fn peek(&self) -> &T;                                   // 借用读，拿不走所有权
    pub fn map<U>(self, f: impl FnOnce(&T) -> U) -> Tainted<U>; // 派生：同集保持
    pub fn join<U, V>(self, other: Tainted<U>, f: impl FnOnce(&T, &U) -> V) -> Tainted<V>; // 并集
    pub fn taint(&self) -> &TaintSet;
}
```

- **无解包面**：无 `into_inner`、无 `Deref`、字段私有——「摘干净再传下游」编译不过（trybuild 反例）。`map` 取 `FnOnce(&T)`（借用入参），闭包无法把所有权搬出环外。
- **C15 拒绝说来源只有一种说法**：`command`、`undoable`、`domain` 三扇门的违规句都写「carries content from {taint}」，经 `TaintSet` 的 `Display` 点出每个来源的标签。落选的是只报个数：个数告诉读者有外来内容，却不告诉他该去查哪一个来源。
- **Tainted 恒不 serde**：`Deserialize` 即第二构造入口，伪造空 Taint 即洗白；`TaintSet` 可 serde（事件载荷需要来源清单）。
- proptest `join_output_contains_both_inputs` 守 `join` 输出 taint ⊇ 两入参（kani 不接手，理由见 `crates/kernel/Spec.lean` §2）。
- 上游错误文本、摘要继承、动作构造器强制并集：均在消费方模块（discard／approval／gate，以及 pipeline／digest）逐处落实，本模块只供类型。
-/
