-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::handoff

规定 `handoff`（`crates/runtime/src/` 下同名的文件）。冻结与续接：五段交接单与它唯一的构造点。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::handoff` 旁的测试守住。
-/

/-!
### 8-5 runtime::handoff（形状 2）


```rust
#[derive(Serialize)]   // 五个字段名即 handoff_written 的五个键；不派生 Deserialize
pub struct Handoff { /* must_read、overview、progress、context、next_step —— 私有 */ }
impl Handoff {
    /// Sole constructor: must-read non-empty; every
    /// entry is an already-parsed Locator by type. Five sections always
    /// present; prose quality is the handoff probe's business, not the type's.
    pub fn new(must_read: Vec<Locator>, overview: String, progress: String,
               context: String, next_step: String) -> Result<Handoff, AxError>;   // 空 must_read → E_INVALID_ARGS
    pub fn must_read(&self) -> &[Locator];  pub fn payload(&self) -> Result<Payload, AxError>;  // handoff_written 载荷
}
pub struct ResumeSeed { pub run: RunId, pub must_read: Vec<Locator> }
/// Resume consumes a Handoff and mints a new identity — never revives the
/// frozen one (元原则六). The caller supplies the new RunId (kernel 禁随机).
pub fn resume(handoff: &Handoff, new_run: RunId) -> ResumeSeed;
```

- **载荷即这五个字段**：`payload` 是 `Payload::of(self)`，键名由字段名给出，不另手写一遍。只派生 `Serialize`：`new` 是持有一个 `Handoff` 的唯一路径，能从一行账本反造一个的读方会绕过它对空 must-read 的拒绝。
- 「下一步」段首列用户指定动作、must-read 规范类机器填：内容约束属生产者（回合层与 spine 文件），类型只强制结构。
- Run<Frozen> 无解冻：resume 不收 Run 值，只收 Handoff——「旧 Run 醒来」在签名上无法拼写。
-/
