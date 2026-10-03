-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::score 与 citysim::metabolism

规定 `citysim::score`（`tools/citysim/src/score.rs`）与 `citysim::metabolism`（`tools/citysim/src/metabolism.rs`），两件只在测试构型里编译的仪器（D21）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-8-2，别处引作 `tools/citysim/Spec.lean §8-8-2`。

能写成定理的是处置的次序：没有任何资产在第一次被注意到的那一轮里退场（`nothing_retires_the_round_it_is_first_noticed`）；一个资产被留下，当且仅当它最近被用过、又付得起它占的地方（`an_asset_is_kept_exactly_when_it_is_used_and_pays`）。两个阈值 `ASSET_IDLE_DAYS`、`ASSET_FLOOR_PER_MILLE` 的值住 `metabolism.rs`，模型把它们当参数（§14）。分数怎样从使用次数与常驻字节算出（`score`）、`worst_first` 的排序由 `score` 的测试守着（§16）。
-/

/-!
#### 8-8-2 score 与 metabolism（形状 1 判定；仅测试构型）

```rust
pub struct AssetUse { pub uses: u32, pub resident: ByteLen, pub idle_days: u32 }
pub struct Score { pub per_mille: u32, pub idle_days: u32 }
pub fn score(usage: &AssetUse) -> Score;
pub fn worst_first<T: Clone>(assets: &[(T, Score)]) -> Vec<(T, Score)>;

pub const ASSET_IDLE_DAYS: u32 = 90;
pub const ASSET_FLOOR_PER_MILLE: u32 = 1_000;
pub enum Disposal { Keep, Warn { because: String }, Retire { because: String } }
impl Disposal { pub fn because(&self) -> &str; }   // Keep 也有一句理由
pub fn dispose(usage: &AssetUse, score: Score, warned_already: bool) -> Disposal;
pub fn sweep<T: Clone>(assets: &[(T, AssetUse, Score, bool)]) -> Vec<(T, Disposal)>;
```

- **分子是被取用次数，分母是常驻字节**：同样的有用程度，占的地方越大越贵——那是它在每一次披露它的 prompt 里都要付的账。不占字节的资产只按次数计分：不占地方的东西不为地方付账。
- **`idle_days` 并列而不折进分数**：便宜且无用与昂贵且不可或缺是两回事。
- **最重的处置是 `Retire`，不是删除**：退场＝不再被披露，字节仍在盘上与历史里。
- **先警告后退场，理由随处置同行**：没有任何东西在第一次被注意到的同一轮里停止被提供——那一轮正是人说「它重要」的机会。
- `worst_first` 先比分数（升），平手时闲置更久的在前，再平手保持调用方的次序：同一份登记两次列出同一张表。`sweep` 按调用方的次序交回处置，不写任何东西：资产的地位只在 `kernel::registry` 改变。
- 平台：Windows、macOS、Linux 上相同，纯计算。
- Rust 检查：`metabolism` 的测试（`a_full_cycle_warns_first_and_retires_second` 对应先警告后退场，`an_asset_that_pays_for_its_room_is_left_alone` 与 `a_heavy_asset_that_is_barely_used_is_noticed_even_while_it_is_fresh` 对应留下的条件）。
-/

namespace Citysim.Metabolism

/-- 一个资产在一段时间里的使用（`score::AssetUse`）：使用次数与常驻字节已折进分数，模型只留处置要读的 `idle_days`。 -/
structure AssetUse where
  idle_days : Nat
  deriving DecidableEq, Repr

/-- 一个资产的分数（`score::Score`）。 -/
structure Score where
  per_mille : Nat
  idle_days : Nat
  deriving DecidableEq, Repr

/-- 两个阈值（`ASSET_IDLE_DAYS`、`ASSET_FLOOR_PER_MILLE`），值住 `metabolism.rs`。 -/
structure Limits where
  idle_days : Nat
  floor_per_mille : Nat

/-- 处置（`metabolism::Disposal`），理由的文字不进模型。 -/
inductive Disposal where
  | Keep
  | Warn
  | Retire
  deriving DecidableEq, Repr

/-- 一个资产的处置（`metabolism::dispose`）：最近被用过、又付得起它占的地方就留下；否则警告过的退场，没警告过的先警告。 -/
def dispose (limits : Limits) (usage : AssetUse) (score : Score) (warned_already : Bool) : Disposal :=
  if usage.idle_days < limits.idle_days ∧ limits.floor_per_mille ≤ score.per_mille then .Keep
  else if warned_already then .Retire
  else .Warn

/-- 没有任何资产在第一次被注意到的那一轮里退场：没被警告过的，最重是 `Warn`。 -/
theorem nothing_retires_the_round_it_is_first_noticed (limits : Limits) (usage : AssetUse)
    (score : Score) :
    dispose limits usage score false ≠ .Retire := by
  by_cases paying : usage.idle_days < limits.idle_days ∧ limits.floor_per_mille ≤ score.per_mille
    <;> simp [dispose, paying]

/-- 一个资产被留下，当且仅当它最近被用过、又付得起它占的地方；被留下与是否警告过无关。 -/
theorem an_asset_is_kept_exactly_when_it_is_used_and_pays (limits : Limits) (usage : AssetUse)
    (score : Score) (warned_already : Bool) :
    dispose limits usage score warned_already = .Keep
      ↔ usage.idle_days < limits.idle_days ∧ limits.floor_per_mille ≤ score.per_mille := by
  by_cases paying : usage.idle_days < limits.idle_days ∧ limits.floor_per_mille ≤ score.per_mille
  · simp [dispose, paying]
  · cases warned_already <;> simp [dispose, paying]

/-- 一个警告过、仍未改善的资产在下一轮退场：先警告后退场是两轮，不会停在警告上。 -/
theorem a_warned_asset_that_did_not_recover_retires (limits : Limits) (usage : AssetUse)
    (score : Score) (noticed : dispose limits usage score false = .Warn) :
    dispose limits usage score true = .Retire := by
  by_cases paying : usage.idle_days < limits.idle_days ∧ limits.floor_per_mille ≤ score.per_mille
    <;> simp_all [dispose]

end Citysim.Metabolism
