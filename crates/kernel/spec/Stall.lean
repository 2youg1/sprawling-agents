-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::stall

规定 `kernel::stall`（`crates/kernel/src/stall.rs`）：原地打转的唯一依据。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::stall` 旁的测试守住。
-/

/-!
### 8-14 kernel::stall

```rust
pub struct ActionFingerprint(B3Hash);        // 动作规范字节的摘要；derive(bytes) 内调 B3Hash::digest
pub enum StallVerdict { Ok, Stall { repeats: u32 } }
/// Sole stall criterion. Sample = recent fingerprints
/// in time order; a tail run of identical prints ≥ LOOP_REPEAT_THRESHOLD
/// is a stall. Counters and queues live with the caller, never here.
pub fn observe(recent: &[ActionFingerprint]) -> StallVerdict;
```

- 判尾部连续：历史中早先的重复不算（已被新动作打断＝已恢复）。阬值取 `LOOP_REPEAT_THRESHOLD`(3)。
- watchdog 只消费 verdict 不转发依据；`E_LOOP_SUSPECTED` 的塑形在处置面。
-/
