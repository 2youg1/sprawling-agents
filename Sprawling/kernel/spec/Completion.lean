-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::completion

规定 `kernel::completion`（`crates/kernel/src/completion.rs`）：一次 run 的结局、能作证的种类与进度。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::completion` 旁的测试守住。
-/

/-!
### 8-20 kernel::completion

```rust
pub struct Evidence(/* Vec<EventRef> 私有 */);
pub(crate) const CITABLE: [EventKind; 3];   // tool_result、model_returned、harness_answered
impl Evidence {
    /// Non-empty and every ref kind ∈ `CITABLE`, else E_EVIDENCE_MISSING.
    /// A6's type half.
    pub fn new(refs: Vec<EventRef>) -> Result<Evidence, AxError>;
    pub fn refs(&self) -> &[EventRef];
}
pub enum Completion { Done(Evidence), Limit, Cancelled }
impl Completion { pub fn name(&self) -> &'static str;
                  pub fn extend_payload(&self, map: &mut Map<String, Value>) -> Result<(), AxError>; }
                  // 单向投影进事件载荷：`completion`，Done 另带证据的 {seq, kind}；刻意不 serde——反序列化会凭空铸出 EventRef

pub struct PlannedProgress { pub done: u32, pub blocked: u32, pub total: u32,
                             pub done_ppb: u64, pub blocked_ppb: u64 }   // 按计划自己的份额加权，十亿分之一（share::WHOLE_PPB）
impl PlannedProgress { pub fn ratio(&self) -> (u32, u32);        // (done, total)；呈现方自算百分比
                       pub fn weighted(&self) -> (u64, u64); }   // (done_ppb, WHOLE_PPB)：按份额而不是按行数
pub struct UnplannedProgress { pub steps: u32, pub budget: BudgetUse }   // 无 ratio 方法：类型层诚实（A17）
pub enum Progress { Planned(PlannedProgress), Unplanned(UnplannedProgress) }
```

- 两态分两 struct 而非 enum 携字段：百分比方法只能长在 Planned 上，Unplanned 拿不到——「界面拿不到百分比就画不出百分比」的类型形态。
- `EventRef` 有 `pub fn kind(&self) -> EventKind`：Evidence 校验需读 kind。
- **能作证的种类只有一张表。** `completion::CITABLE` 列出 `tool_result`、`model_returned` 与 `harness_answered`；`Evidence::new` 与 `registry::Artifact::verify` 读同一张表，拒绝时的恢复语也由这张表拼出，不另写一遍三个词。作证与窗类是两件事：`harness_answered` 是 record-only，它能作证，是因为它是城自己记下的、harness 对城那次请求的回答，与模型 run 以最后一条 `model_returned` 作证同一标准。`harness_reported` 恒不作证：汇报是 harness 自己说的话，城没有判过它（性质见 `HarnessRun.lean` 的 `a_cited_record_is_admitted`）。
-/
