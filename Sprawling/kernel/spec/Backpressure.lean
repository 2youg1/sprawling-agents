-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::backpressure

规定 `kernel::backpressure`（`crates/kernel/src/backpressure.rs`）：队列的准入。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-13 kernel::backpressure

```rust
pub struct QueueStats { pub depth: u64, pub capacity: u64 }
pub struct ItemMeta { pub cost: u64 }        // 槽位数：Signal＝1，受理新 Run 的 fd 预留可 >1
pub enum ShedReason { CapacityExhausted }
pub enum Admission { Admit, Shed { reason: ShedReason } }
/// Decides whether the queue admits one more item. Pure and total:
/// depth + cost ≤ capacity admits; checked arithmetic, overflow sheds.
pub fn admit(stats: &QueueStats, item: &ItemMeta) -> Admission;
```

- 削峰是 city-wide 准入姿态：同一函数服务 Signal 队列与 fd 预留（capacity 语义由调用方赋）；队列与计数器住 storage::queue（S3）与调用方。
- kani：全函数无溢出；单调性——同 capacity/cost 下 depth 更小恒不更难 Admit。
- 饱饱不饿死（活性）属 citysim liveness（P2），非本函数可证。
-/

namespace Kernel.Backpressure

/-- 与 `kernel::ShedReason` 逐变体同名。 -/
inductive ShedReason where
  | CapacityExhausted
  deriving DecidableEq, Repr

/-- 与 `kernel::Admission` 逐变体同名。 -/
inductive Admission where
  | Admit
  | Shed (reason : ShedReason)
  deriving DecidableEq, Repr

/-- `backpressure::admit`：`depth + cost` 不溢出且不超过容量即收下。`bound` 是 `u64` 能装下的值的个数，`checked_add` 在和达到它时失败，失败即削。 -/
def admit (bound depth cost capacity : Nat) : Admission :=
  if depth + cost < bound ∧ depth + cost ≤ capacity then .Admit else .Shed .CapacityExhausted

/-- 收下的都在容量之内。 -/
theorem admitted_items_fit (bound depth cost capacity : Nat)
    (admitted : admit bound depth cost capacity = .Admit) : depth + cost ≤ capacity := by
  simp only [admit] at admitted
  split at admitted
  · rename_i fits
    exact fits.2
  · cases admitted

/-- **溢出的和恒被削**，对任意容量成立（kani 的 `an_overflowing_sum_never_admits` 在真实 MIR 上证同一条）。 -/
theorem an_overflowing_sum_never_admits (bound depth cost capacity : Nat)
    (overflows : bound ≤ depth + cost) :
    admit bound depth cost capacity = .Shed .CapacityExhausted := by
  simp only [admit]
  split
  · rename_i fits
    omega
  · rfl

/-- **队列越短越不难收**：同容量同成本下，深度更小的恒不更难收下（kani 的 `admit_is_total_and_monotone_in_depth`）。 -/
theorem admit_is_monotone_in_depth (bound shallow deep cost capacity : Nat) (shallower : shallow ≤ deep)
    (admitted : admit bound deep cost capacity = .Admit) :
    admit bound shallow cost capacity = .Admit := by
  simp only [admit] at *
  split at admitted
  · rename_i fits
    split
    · rfl
    · rename_i misses
      omega
  · cases admitted

end Kernel.Backpressure
