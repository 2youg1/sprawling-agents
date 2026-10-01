-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::suite

规定 `citysim::suite`（`tools/citysim/src/suite.rs`），以及 §8-8 五件仪器共有的规矩。`score`、`metabolism` 在 `spec/Metabolism.lean`，`nesting` 在 `spec/Nesting.lean`，`ablation` 在 `spec/Ablation.lean`。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面各节保留它们在 citysim 规格里的标签 §8-8、§8-8-1，别处引作 `tools/citysim/Spec.lean §8-8`。

能写成定理的是一份报告的计数：每个 outcome 恰好记一次，记进它那一半的 `tried`，或记进 `unknown`（`every_outcome_is_counted_once`）；不认识的 outcome 不进任何一半的分母（`an_outcome_nobody_asked_for_is_unknown`）。「同一个 id 两次即拒」由 Rust 的 `Suite::new` 持有，模型把建好的 suite 当作 id 到一半的函数，于是一个 id 在模型里只能属于一半。
-/

/-!
### 8-8 仪器：suite、score、metabolism、nesting、ablation

五件仪器回答「城拿什么证据评估自己」。它们**恒不是合并门**：一件仪器说某样东西变差了，是给人看的证据，不是 CI 的红灯。量的是模型行为，两次不一样是常态，所以它们出证据不出红灯；设阈值的门归 `xtask budget`，依据是「机器两次量得一样」。

| 模块 | 它回答的问题 | 构型 |
|---|---|---|
| `suite`（含 held-out 判定） | 一批真实任务怎么组织、怎么跑两次而结果可比 | 库面，`tests/evaluation.rs` 驱动 |
| `score`、`metabolism` | 哪些沉淀资产在升值、哪些该退场 | 仅测试构型 |
| `nesting` | 模型编辑哪种嵌套格式错得最少，错时怎么错 | 仅测试构型 |
| `ablation` | 拿掉 City.md 的某一段，居民做不了什么 | 仅测试构型 |

三条前提只消费不重议：**评分对象是资产不是 Agent**（会话冻结即终结，Ephemeral 恒不进评分与 metabolism）；**语料只取自真实工作**（一份合成任务集测出来的分数，测的是出题人）；**登记归 `kernel::registry`**（Asset 是什么、登记在哪由它答；这里只答「这份登记值多少」，成本读数归 `storage::attribution`）。统计全用整数，比率以千分数（`per_mille`）表达，不引入统计库。

D21 **仪器只在测试构型里编译。** `score`、`metabolism`、`nesting`、`ablation` 在 `lib.rs` 写作 `#[cfg(test)] mod`：它们回答的是「这套规则算得对不对」，答法是自己的测试，提问者是读测试的人；没有剧本调用它们。dead_code 因此不是被 `#[allow]` 压掉的，是不存在的。被否：四件都编进库面——它们没有调用者，库要么背着一片 dead_code，要么用 `#[allow]` 压住它。**重开条件**：出现一个生产调用点要对资产排序或退场，例如城层的资产清单视图；届时那个模块搬到拥有该视图的 crate。
-/

/-!
#### 8-8-1 suite（形状 2 值类型＋形状 1 判定）

```rust
pub enum Half { HeldIn, HeldOut }
pub struct Task { pub id: String, pub at: Locator, pub half: Half }
pub struct Outcome { pub id: String, pub passed: bool }
pub struct Tally { pub tried: u32, pub passed: u32 }   // per_mille() 整数千分比
pub struct Report { pub held_in: Tally, pub held_out: Tally, pub unknown: u32 }
pub struct Suite { /* BTreeMap<String, Task> —— 私有 */ }
impl Suite {
    pub fn new(tasks: Vec<Task>) -> Result<Suite, AxError>;   // 空 id 与同一 id 两次即拒（泄漏在构造点）
    pub fn half(&self, half: Half) -> Vec<&Task>;             // id 序＝执行序
    pub fn report(&self, outcomes: &[Outcome]) -> Report;
}
```

- **泄漏是构造点的拒绝，不是事后的告警**：同一个 id 出现两次即拒，无论落在同半还是异半。一份被看过的 held-out 集在它被看过之后就不值钱了。没有 id 的任务同样在构造点被拒：结果按 id 对。
- **任务只携 Locator 不携正文**：抄一份正文进来就会与它来自的那件活漂开。
- **不认识的 outcome 计入 `unknown` 而非计入分母**：一次回答了没人问过的问题的运行，不是这份 suite 的运行。
- 一半里没有一个任务被试过时，千分比是 0 而不是一次除零（`Tally::per_mille`）。
-/

namespace Citysim.Suite

/-- 一个任务属于哪一半（`suite::Half`）。 -/
inductive Half where
  | HeldIn
  | HeldOut
  deriving DecidableEq, Repr

/-- 一个任务在一次运行里的结果（`suite::Outcome`）。 -/
structure Outcome where
  id : String
  passed : Bool
  deriving DecidableEq, Repr

/-- 一半的计数（`suite::Tally`）。 -/
structure Tally where
  tried : Nat
  passed : Nat
  deriving DecidableEq, Repr

/-- 一次运行的报告（`suite::Report`）。 -/
structure Report where
  held_in : Tally
  held_out : Tally
  unknown : Nat
  deriving DecidableEq, Repr

/-- 建好的 suite：每个 id 属于哪一半，或不在 suite 里。`Suite::new` 拒掉同一 id 两次之后，这就是一个函数。 -/
abbrev Suite := String → Option Half

/-- 把一个结果记进一半的计数。 -/
def Tally.record (tally : Tally) (passed : Bool) : Tally :=
  ⟨tally.tried + 1, if passed then tally.passed + 1 else tally.passed⟩

/-- 把一个 outcome 记进报告（`Suite::report` 的循环体）。 -/
def Report.record (suite : Suite) (report : Report) (outcome : Outcome) : Report :=
  match suite outcome.id with
  | none => { report with unknown := report.unknown + 1 }
  | some .HeldIn => { report with held_in := report.held_in.record outcome.passed }
  | some .HeldOut => { report with held_out := report.held_out.record outcome.passed }

/-- 一次运行的报告（`Suite::report`）。 -/
def report (suite : Suite) (outcomes : List Outcome) (from_ : Report) : Report :=
  outcomes.foldl (Report.record suite) from_

/-- 每个 outcome 恰好记一次：两半的 `tried` 与 `unknown` 加起来恰是 outcome 的条数。 -/
theorem every_outcome_is_counted_once (suite : Suite) (outcomes : List Outcome) (from_ : Report) :
    let after := report suite outcomes from_
    after.held_in.tried + after.held_out.tried + after.unknown
      = from_.held_in.tried + from_.held_out.tried + from_.unknown + outcomes.length := by
  induction outcomes generalizing from_ with
  | nil => simp [report]
  | cons outcome rest ih =>
    have := ih (Report.record suite from_ outcome)
    simp only [report, List.foldl_cons] at this ⊢
    rw [this]
    simp only [Report.record, List.length_cons]
    split <;> simp [Tally.record] <;> omega

/-- 一个不在 suite 里的 outcome 只记进 `unknown`，不动任何一半的分母。 -/
theorem an_outcome_nobody_asked_for_is_unknown (suite : Suite) (report : Report) (outcome : Outcome)
    (stranger : suite outcome.id = none) :
    Report.record suite report outcome = { report with unknown := report.unknown + 1 } := by
  simp [Report.record, stranger]

end Citysim.Suite
