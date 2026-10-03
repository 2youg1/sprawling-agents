-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::red_team

规定 `citysim::red_team`（`tools/citysim/src/red_team.rs`）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-7，别处引作 `tools/citysim/Spec.lean §8-7`。

能写成定理的是两臂的差别（D19）：未验证臂留下每一条结论；验证臂只留下引文成立的那些，所以当判定把每一种埋下的缺陷读成它自己那种不成立、把忠实的结论读成成立时，验证臂放行的缺陷为 0、误删的忠实结论为 0。判定本身是 `collab::Citation::against`，它怎样读出这些结果是 collab 的事，模型把「判定忠实」写成定理的假设，`every_planted_defect_is_dropped_by_its_own_reading` 在真实的判定上检查它。
-/

/-!
### 8-7 红队：有无验证 run 两臂的结论质量（`citysim::red_team`）

形状：decision。红队剧本是一组固定的 `Case`：每个 case 是一份草稿，外加作者 run 交出的结论，每条结论带一条 `collab::Citation` 与红队写下的真相 `Plant`（`Faithful`，或三种埋下的缺陷 `Misquote`／`OtherVersion`／`PastEnd`，与 `collab::Reading` 的三种不成立读数一一对应）。剧本就是脚本化 provider 在这里的角色：同一剧本两臂各跑一次，逐字节可复跑。

```rust
pub enum Arm { Unverified, Verified }
pub enum Plant { Faithful, Misquote, OtherVersion, PastEnd }
pub struct Claim { pub citation: Citation, pub plant: Plant }
pub struct Case { pub draft: String, pub claims: Vec<Claim> }
pub struct Tally { pub kept_faithful: usize, pub kept_planted: usize, pub dropped_faithful: usize, pub dropped_planted: usize }
impl Tally { pub fn precision_per_mille(&self) -> Option<usize> }
pub struct Comparison { pub unverified: Tally, pub verified: Tally }
pub fn compare(cases: &[Case]) -> Comparison
```

- `Arm::Unverified` 留下作者交出的每条结论；`Arm::Verified` 把每条引文对草稿钉住的版本（`cas:` 草稿全文摘要、无区间）跑一次 `Citation::against`，只留 `Reading::Holds` 的结论。
- 结论质量＝留下的结论里忠实者的千分比（`precision_per_mille`）；一条都没留下时为 `None`，因为零分之零不是质量。另两格（误删的忠实结论、放行的缺陷）照实计数，使验证 run 的代价与收益在同一张表上。
- 无失败出口：一条不成立的引文是验证 run 要报的结果，不是故障（与 `collab::Reading` 同一口径）。

- 平台：Windows、macOS、Linux 上相同，纯计算。

Rust 检查（`red_team::tests`）：`the_verified_arm_keeps_only_faithful_conclusions`——同一剧本下，验证臂的千分比为 1000、放行缺陷为 0，未验证臂低于它；`every_planted_defect_is_dropped_by_its_own_reading`——三种埋下的缺陷各自被验证臂删掉，忠实结论一条不误删。
-/

namespace Citysim.RedTeam

/-- 一条结论属于哪一臂（`red_team::Arm`）。 -/
inductive Arm where
  | Unverified
  | Verified
  deriving DecidableEq, Repr

/-- 红队埋下的真相（`red_team::Plant`），两臂都不读它。 -/
inductive Plant where
  | Faithful
  | Misquote
  | OtherVersion
  | PastEnd
  deriving DecidableEq, Repr

/-- 判定对一条引文的读数（`collab::Reading`，正文差异的细节不进模型）。 -/
inductive Reading where
  | Holds
  | OtherVersion
  | OutOfRange
  | Differs
  deriving DecidableEq, Repr

/-- 一条结论：埋下的真相，与判定对它引文的读数。 -/
structure Claim where
  plant : Plant
  reading : Reading
  deriving DecidableEq, Repr

/-- 每一臂的四格（`red_team::Tally`）。 -/
structure Tally where
  kept_faithful : Nat
  kept_planted : Nat
  dropped_faithful : Nat
  dropped_planted : Nat
  deriving DecidableEq, Repr

/-- 一臂留不留一条结论（`red_team::verdict`）：两臂只差判定是否被调用。 -/
def kept : Arm → Claim → Bool
  | .Unverified, _ => true
  | .Verified, claim => claim.reading == .Holds

/-- 把一条结论记进它的那一格（`Tally::count`）。 -/
def Tally.count (tally : Tally) (plant : Plant) (keep : Bool) : Tally :=
  match keep, plant with
  | true, .Faithful => { tally with kept_faithful := tally.kept_faithful + 1 }
  | true, _ => { tally with kept_planted := tally.kept_planted + 1 }
  | false, .Faithful => { tally with dropped_faithful := tally.dropped_faithful + 1 }
  | false, _ => { tally with dropped_planted := tally.dropped_planted + 1 }

/-- 一臂跑完全部结论的四格（`red_team::tally`）。 -/
def tally (arm : Arm) (claims : List Claim) (from_ : Tally) : Tally :=
  claims.foldl (fun sum claim => sum.count claim.plant (kept arm claim)) from_

/-- 判定忠实：一条结论的引文成立，当且仅当它是忠实的。这是 `collab::Citation::against` 的性质，不是本模型的；它在模型里是假设。 -/
def FaithfulJudge (claims : List Claim) : Prop :=
  ∀ claim ∈ claims, claim.reading = .Holds ↔ claim.plant = .Faithful

/-- 未验证臂留下每一条结论：什么都不删。 -/
theorem the_unverified_arm_drops_nothing (claims : List Claim) (from_ : Tally) :
    (tally .Unverified claims from_).dropped_faithful = from_.dropped_faithful
      ∧ (tally .Unverified claims from_).dropped_planted = from_.dropped_planted := by
  induction claims generalizing from_ with
  | nil => simp [tally]
  | cons claim rest ih =>
    have := ih (from_.count claim.plant true)
    simp only [tally, List.foldl_cons, kept] at this ⊢
    rw [this.1, this.2]
    cases claim.plant <;> simp [Tally.count]

/-- D19 **两臂共用同一份剧本与同一个判定函数 `collab::Citation::against`，只差「判定是否被调用」。** 落选的做法是在 `suite`（§8-8-1）里建套件：`Suite` 量的是 held-in／held-out 的通过率，没有「同一结论集、去掉一个环节」这一维，而且 citysim 已经有固定剧本与计数时钟。真实 provider 的读数替换的是剧本里的作者，不是判定；判定可复跑，所以 CI 只跑脚本化这一侧。

判定忠实时，验证臂一条缺陷都不放行，一条忠实结论都不误删。 -/
theorem the_verified_arm_keeps_exactly_the_faithful (claims : List Claim) (from_ : Tally)
    (judge : FaithfulJudge claims) :
    (tally .Verified claims from_).kept_planted = from_.kept_planted
      ∧ (tally .Verified claims from_).dropped_faithful = from_.dropped_faithful := by
  induction claims generalizing from_ with
  | nil => simp [tally]
  | cons claim rest ih =>
    have rest_judged : FaithfulJudge rest := fun c member => judge c (List.mem_cons_of_mem _ member)
    have here := judge claim List.mem_cons_self
    have := ih (from_.count claim.plant (kept .Verified claim)) rest_judged
    simp only [tally, List.foldl_cons] at this ⊢
    rw [this.1, this.2]
    cases plant : claim.plant with
    | Faithful =>
      have holds : claim.reading = .Holds := here.mpr plant
      simp [kept, holds, Tally.count]
    | Misquote | OtherVersion | PastEnd =>
      have fails : claim.reading ≠ .Holds := fun holds => by simp [here.mp holds] at plant
      have unkept : (claim.reading == Reading.Holds) = false := by simpa using fails
      simp [kept, unkept, Tally.count]

end Citysim.RedTeam
