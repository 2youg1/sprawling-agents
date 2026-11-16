-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::repair

规定 `kernel::repair`（`crates/kernel/src/repair.rs`）：修复的租约。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::repair` 旁的测试守住。
-/

/-!
### 8-16 kernel::repair

```rust
pub enum RepairVerdict { Lease, Queued { holder: RunId } }
/// One live lease per scope subtree: overlap either
/// way queues; the same holder re-requesting its exact scope re-leases
/// (idempotent). State (the active map) lives with the caller.
pub fn request(active: &BTreeMap<Address, RunId>, scope: &Address, who: &RunId) -> RepairVerdict;
```

- 重叠依据同 goal：`scope.is_within(s) || s.is_within(scope)`；报首个重叠的 holder（BTreeMap 序确定）。
-/
