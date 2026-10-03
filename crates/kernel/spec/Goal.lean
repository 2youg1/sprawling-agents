-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::goal

规定 `kernel::goal`（`crates/kernel/src/goal.rs`）：目标与同资源的冲突检测。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::goal` 旁的测试守住。
-/

/-!
### 8-15 kernel::goal

```rust
pub struct GoalId(String);                   // 非空
pub enum GoalResource { Path(Address), External(String) }  // External 非空（外部不可分资源名）
pub struct GoalEntry { pub id: GoalId, pub owner: String, pub resources: Vec<GoalResource>,
                       pub statement: String, pub standing: bool }
pub enum GoalVerdict { Clear, Conflict { with: GoalId } }
/// Same-resource mutual exclusion only: detection is
/// kernel's, arbitration is not. Paths conflict on prefix overlap either
/// way; External conflicts on equality; Path vs External never.
pub fn detect_conflict(registered: &[GoalEntry], candidate: &GoalEntry) -> GoalVerdict;
```

- 报首冲突（registered 切片序，确定）；id 去重归登记方（调用方持表）；candidate 自冲突不判（同 owner 同 id 重提交属幂等）。
- `E_GOAL_CONFLICT` 的塑形在注册回传（工具面）；kernel 只出 verdict。
-/
