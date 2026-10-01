-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::idem

规定 `kernel::idem`（`crates/kernel/src/idem.rs`）：外向动作的去重键与判重凭证。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-6 kernel::idem

```rust
pub struct IdemKey { v: u8, digest: [u8; 16] }   // 私有；无 From<Uuid>、无 Default、无随机
pub const IDEM_DERIVE_V: u8 = 1;
impl IdemKey {
    /// Deterministic dedup key for outward actions:
    /// BLAKE3-XOF 16 bytes over `run(16B) || seq(8B LE) || action_canonical`.
    /// Fixed-width prefix makes the framing injective; same inputs after
    /// resume/replay re-derive the identical key.
    pub fn derive(run: &RunId, seq: Seq, action_canonical: &[u8]) -> IdemKey;
}
// 定义点仍在 kernel::idem；取关联函数而非自由函数，避免裸名 `derive` 入 crate 门面。
impl fmt::Display for IdemKey { /* "idem<v>-<hex32>" */ }

pub struct IdemGuard { key: IdemKey }            // 私有字段；唯一铸口是 claim
impl IdemGuard { pub fn key(&self) -> &IdemKey; }
pub struct Duplicate { key: IdemKey }            // 已被认领过的键，供调用方取回第一次的答案
impl Duplicate { pub fn key(&self) -> &IdemKey; }
/// Claims the key in the caller's seen set: Ok grants the right to act once.
pub fn claim(seen: &mut BTreeSet<IdemKey>, key: IdemKey) -> Result<IdemGuard, Duplicate>;
```

- **`IdemGuard` 是判重的凭证**：它把「在任何不可重放的副作用之前判重」这条顺序约束变成编译期约束：副作用入口收 `&IdemGuard`，而 guard 只有 `claim` 造得出。seen 集合仍是调用方的状态，kernel 不持有任何状态。

serde：字符串形。动作规范化（action_canonical 的构造规则）属工具面——它在那一面有且只有一个实现，`ToolCall::action`（§8-23）；本模块只定派生函数与框架。
-/

namespace Kernel.Idem

/-- `idem::claim`：键已在调用方的 seen 集里即 `Duplicate`（带着那把键，供调用方取回第一次的答案）；否则记下它并交出凭证。seen 集是调用方的状态，这里是一张表。 -/
def claim {Key : Type} [DecidableEq Key] (seen : List Key) (key : Key) : Except Key (List Key) :=
  if key ∈ seen then .error key else .ok (key :: seen)

/-- 没见过的键领到凭证。 -/
theorem a_fresh_key_is_granted {Key : Type} [DecidableEq Key] (seen : List Key) (key : Key)
    (fresh : key ∉ seen) : claim seen key = .ok (key :: seen) := by
  simp [claim, fresh]

/-- **一把键只领得到一次凭证。** 副作用入口收 `&IdemGuard`，凭证只有 `claim` 造得出，所以同一个动作在任何不可重放的副作用之前恰被判重一次（`ToolBench::invoke` 的 `dedup_runs_before_the_side_effect`）。 -/
theorem a_key_is_granted_once {Key : Type} [DecidableEq Key] (seen after : List Key) (key : Key)
    (granted : claim seen key = .ok after) : claim after key = .error key := by
  simp only [claim] at granted
  split at granted
  · cases granted
  · cases granted
    simp [claim]

end Kernel.Idem
