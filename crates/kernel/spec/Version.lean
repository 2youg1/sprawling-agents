-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::version

规定 `kernel::version`（`crates/kernel/src/version.rs`）：乐观并发的版本与新鲜度判定。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-5 kernel::version

```rust
pub struct Version(u64);
impl Version { pub const FIRST: Version;               // 1；首个可见版本
               pub fn next(self) -> Result<Version, AxError>; }
pub enum VersionVerdict { Fresh, Stale { current: Version } }
/// Optimistic-concurrency primitive: pure verdict, no bool.
pub fn check_base(current: Version, base: Version) -> VersionVerdict;
```

`Stale` 到 `E_VERSION_CONFLICT`＋新鲜 diff 的映射在 runtime::tools::edit：kernel 只判新鲜度，不认识 diff。`base > current` 同样 `Stale`（唯一真版本是 current；超前的 base 是调用方脑补）。
-/

namespace Kernel.Version

/-- 与 `kernel::VersionVerdict` 逐变体同名；版本 `Version` 在模型里是它的数。 -/
inductive VersionVerdict where
  | Fresh
  | Stale (current : Nat)
  deriving DecidableEq, Repr

/-- `check_base`：乐观并发的原语，纯判定，不是布尔。 -/
def check_base (current base : Nat) : VersionVerdict :=
  if base = current then .Fresh else .Stale current

/-- 新鲜当且仅当调用方所基于的版本就是当前版本。 -/
theorem fresh_exactly_at_the_current_version (current base : Nat) :
    check_base current base = .Fresh ↔ base = current := by
  simp only [check_base]
  split <;> simp_all

/-- 超前的 base 同样是陈旧的：唯一的真版本是 current，超前的 base 是调用方脑补的；陈旧的答案带回当前版本。 -/
theorem a_base_ahead_is_stale (current base : Nat) (ahead : current < base) :
    check_base current base = .Stale current := by
  simp only [check_base]
  split
  · omega
  · rfl

end Kernel.Version
