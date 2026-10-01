-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# playback 的选择：哪些行进 bundle、按什么次序、读到哪一行为止

规定 `crates/accounting/src/playback/select.rs` 与 `crates/accounting/src/playback/walk.rs`（accounting-SPEC.md 8-12）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一次导出先固定 cutoff：严格校验（`storage::LineCheck`）走到的最后一个完整行。范围内事件恰好是「seq 不超过 cutoff」与选择条件的交集；选择条件是 seq 的闭区间 `[first, last]`、一个 run、一栋楼、一个 UTC 时间的半开区间 `[since, until)`（accounting-SPEC.md 8-17），给了几项就取几项的交集。时间条件比的是每一行信封的 `t`，而 `t` 不随 seq 单调，所以它与别的条件一样逐行判断。

五条性质，各一组定理：

* **选择是交集，不多不少**（`mem_selected`）；
* **次序与去重**：LineCheck 让账本的 seq 严格递增（`Chained`），被选中的行仍严格递增，所以每个 seq 恰好出现一次（`selected_chained`、`selected_seqs_nodup`）；
* **截止线不读未来**：cutoff 之后追加的行改变不了这次选择（`cutoff_ignores_future`）；
* **逐行判断**：一行在不在选择里只看它自己，与它前后的行无关（`selection_is_per_line`），所以 `t` 回退的行照样按自己的时刻取舍，读者不能在第一条越过 `until` 的行处停下（`a_line_whose_time_steps_back_is_judged_on_its_own`）；
* **矛盾的范围与合法的空选择是两件事**：`first > last` 与 `before ≤ since`（Rust 的 `until ≤ since`）由 Rust 拒绝（`Contradictory`），合法而为空的选择输出带范围信息的空 bundle（`empty_selection_is_legal`）。

模型边界：楼的判定在 Rust 里是 `Address::is_within`（按段边界的前缀），这里抽象成楼的编号相等，`lab` 与 `laboratory` 的区别由 `is_within` 自己的测试守住（kernel-SPEC 8-2）。本模型不是 Rust 实现的证明。两者的一致靠一张场景表：`history` 与 `scenes` 只写在本文件里，`scenes_agree` 证明模型对每一项给出表里的 seq；`crates/accounting/src/playback/tests/model.rs` 从本文件逐行读出 `line …` 与 `scene …` 两种行，按同一张表写账本、跑生产的 `export`，比较选中的 seq（accounting-SPEC.md 8-12、§12-25(i)）。所以这三种行的写法是那个测试读的格式：一行一项，`line seq run building t`，`scene first last run building cutoff [seq, …]`，`window since before cutoff [seq, …]`，`some n` 或 `none`。`--day` 在 Rust 里展开成一个 `window`，不在本模型里。
-/

namespace Accounting.Playback.Select

/-- 账本的一行，只留选择读的字段：seq、run、信封地址所在的楼（没有地址时为 `none`）、信封的 `t`（毫秒）。 -/
structure Line where
  seq : Nat
  run : Nat
  building : Option Nat
  t : Nat
  deriving Repr, DecidableEq

/-- 一次选择：seq 闭区间的两端、一个 run、一栋楼、时间半开区间的两端；`none` 表示这一项不限。时间的上界在 Rust 里叫 `until`，在这里叫 `before`，因为 `until` 是 Lean 的关键字。 -/
structure Selection where
  first : Option Nat
  last : Option Nat
  run : Option Nat
  building : Option Nat
  since : Option Nat
  before : Option Nat
  deriving Repr, DecidableEq

/-- 一行是否满足选择的每一项。没有地址的行不属于任何一栋楼；时间区间含 `since`、不含 `before`。 -/
def Selection.admits (s : Selection) (l : Line) : Bool :=
  s.first.all (· ≤ l.seq) && s.last.all (l.seq ≤ ·) &&
    s.run.all (· == l.run) && s.building.all (fun b => l.building == some b) &&
    s.since.all (· ≤ l.t) && s.before.all (l.t < ·)

/-- 一个区间的两端是否矛盾：seq 区间倒置，或时间区间的上界不晚于下界（`runtime::clock::UtcSpan::new` 的拒绝）。Rust 以 `E_INVALID_ARGS` 拒绝它，不输出 bundle。 -/
def Selection.Contradictory (s : Selection) : Prop :=
  (∃ a b, s.first = some a ∧ s.last = some b ∧ b < a) ∨
    (∃ a b, s.since = some a ∧ s.before = some b ∧ b ≤ a)

/-- 范围内事件：cutoff 以内且满足选择的行，保持账本的次序。 -/
def selected (s : Selection) (cutoff : Nat) (history : List Line) : List Line :=
  history.filter (fun l => decide (l.seq ≤ cutoff) && s.admits l)

/-- LineCheck 走过的历史：seq 严格递增。 -/
def Chained (history : List Line) : Prop :=
  history.Pairwise (fun a b => a.seq < b.seq)

/-! ## 选择是交集，不多不少 -/

theorem mem_selected (s : Selection) (cutoff : Nat) (history : List Line) (l : Line) :
    l ∈ selected s cutoff history ↔ l ∈ history ∧ l.seq ≤ cutoff ∧ s.admits l = true := by
  simp [selected, List.mem_filter]

/-! ## 次序与去重 -/

theorem selected_chained (s : Selection) (cutoff : Nat) (history : List Line)
    (h : Chained history) : Chained (selected s cutoff history) :=
  List.Pairwise.filter _ h

theorem selected_seqs_nodup (s : Selection) (cutoff : Nat) (history : List Line)
    (h : Chained history) : ((selected s cutoff history).map (·.seq)).Nodup := by
  have hc := selected_chained s cutoff history h
  unfold Chained at hc
  rw [List.Nodup, List.pairwise_map]
  exact hc.imp (fun hlt => Nat.ne_of_lt hlt)

/-! ## 截止线不读未来 -/

theorem cutoff_ignores_future (s : Selection) (cutoff : Nat) (history future : List Line)
    (later : ∀ l ∈ future, cutoff < l.seq) :
    selected s cutoff (history ++ future) = selected s cutoff history := by
  have none : future.filter (fun l => decide (l.seq ≤ cutoff) && s.admits l) = [] := by
    rw [List.filter_eq_nil_iff]
    intro l hl
    have := later l hl
    simp [Nat.not_le.mpr this]
  simp [selected, List.filter_append, none]

/-! ## 逐行判断 -/

/-- 一行在不在选择里只看它自己：把历史从任何一处切开，两段各自选出的行接起来就是整段选出的行。所以读者不能因为前面某一行的 `t` 已经越过 `until` 就不再读后面的行。 -/
theorem selection_is_per_line (s : Selection) (cutoff : Nat) (front back : List Line) :
    selected s cutoff (front ++ back) = selected s cutoff front ++ selected s cutoff back := by
  simp [selected, List.filter_append]

/-- `t` 回退的行按它自己的时刻取舍：seq 3 的 `t` 早于 seq 2，seq 4 越过了 `until`，seq 5 又回到区间里，仍被选中。 -/
theorem a_line_whose_time_steps_back_is_judged_on_its_own :
    selected ⟨none, none, none, none, some 20, some 40⟩ 5
      [⟨0, 0, none, 0⟩, ⟨1, 1, some 0, 10⟩, ⟨2, 1, some 0, 30⟩, ⟨3, 1, some 0, 20⟩,
        ⟨4, 1, some 0, 40⟩, ⟨5, 1, some 0, 25⟩] =
      [⟨2, 1, some 0, 30⟩, ⟨3, 1, some 0, 20⟩, ⟨5, 1, some 0, 25⟩] := by
  decide

/-! ## 合法的空选择 -/

/-- 一段城里没有的 run 选出空表，一个没有行落进去的时间区间也选出空表，而两者都不矛盾：Rust 输出空 bundle，范围信息照写。 -/
theorem empty_selection_is_legal :
    let s : Selection := ⟨some 1, some 3, some 9, none, some 0, some 100⟩
    selected s 5 [⟨0, 0, none, 0⟩, ⟨1, 1, some 0, 1⟩, ⟨2, 1, some 0, 2⟩] = [] ∧
      ¬ s.Contradictory := by
  refine ⟨by decide, ?_⟩
  rintro (⟨a, b, ha, hb, hlt⟩ | ⟨a, b, ha, hb, hle⟩) <;> simp at ha hb <;> omega

/-- 时间区间的上界不晚于下界是矛盾的，与 seq 区间倒置同样被拒绝，而不是输出一份空 bundle。 -/
theorem an_empty_span_is_contradictory :
    (⟨none, none, none, none, some 40, some 40⟩ : Selection).Contradictory :=
  Or.inr ⟨40, 40, rfl, rfl, Nat.le_refl 40⟩

/-- 实现可达：一个真实的选择选中范围内、楼内、时间区间内的行，漏掉别的楼的行、区间外的行与 cutoff 之后的行。 -/
theorem a_selection_reaches_lines :
    selected ⟨some 1, none, none, some 0, some 5, none⟩ 3
      [⟨0, 0, none, 0⟩, ⟨1, 1, some 0, 1⟩, ⟨2, 2, some 1, 7⟩, ⟨3, 1, some 0, 9⟩,
        ⟨4, 1, some 0, 9⟩] =
      [⟨3, 1, some 0, 9⟩] := by
  decide

/-! ## 与 Rust 比较的场景表 -/

/-- 一行账本的写法，供场景表逐行书写。 -/
def line (seq run : Nat) (building : Option Nat) (t : Nat) : Line := ⟨seq, run, building, t⟩

/-- 一项场景：一个选择、一个 cutoff、模型应选中的 seq。 -/
structure Scene where
  selection : Selection
  cutoff : Nat
  seqs : List Nat
  deriving Repr, DecidableEq

/-- 一项不限时间的场景的写法，供场景表逐行书写。 -/
def scene (first last run building : Option Nat) (cutoff : Nat) (seqs : List Nat) : Scene :=
  ⟨⟨first, last, run, building, none, none⟩, cutoff, seqs⟩

/-- 一项只限时间的场景的写法：`[since, until)`，其余条件不限。 -/
def window (since before : Option Nat) (cutoff : Nat) (seqs : List Nat) : Scene :=
  ⟨⟨none, none, none, none, since, before⟩, cutoff, seqs⟩

/-- 场景表的账本：seq 0 是创世行；run 1 在楼 0，run 2 在楼 1 且有一行没有地址，run 3 在楼 1。`t` 不随 seq 单调：seq 3 与 seq 5 回退。 -/
def history : List Line := [
  line 0 0 none 0,
  line 1 1 (some 0) 10,
  line 2 2 (some 1) 30,
  line 3 1 (some 0) 20,
  line 4 1 (some 0) 40,
  line 5 2 none 25,
  line 6 3 (some 1) 50,
  line 7 2 (some 1) 60
]

/-- 场景表：每一项单独改一个条件，或在端点、cutoff 与空选择上取边界。 -/
def scenes : List Scene := [
  scene none none none none 7 [0, 1, 2, 3, 4, 5, 6, 7],
  scene none none none none 3 [0, 1, 2, 3],
  scene (some 1) (some 3) none none 7 [1, 2, 3],
  scene (some 3) (some 3) none none 7 [3],
  scene (some 5) none none none 4 [],
  scene none (some 9) none none 7 [0, 1, 2, 3, 4, 5, 6, 7],
  scene none none (some 1) none 7 [1, 3, 4],
  scene none none (some 2) none 6 [2, 5],
  scene none none none (some 0) 7 [1, 3, 4],
  scene none none none (some 1) 7 [2, 6, 7],
  scene (some 2) none (some 2) (some 1) 7 [2, 7],
  scene none none (some 0) none 7 [0],
  scene none none (some 9) none 7 [],
  scene (some 4) (some 6) none (some 1) 7 [6],
  scene none none (some 1) (some 1) 7 [],
  scene (some 0) (some 0) none none 0 [0],
  window (some 20) (some 40) 7 [2, 3, 5],
  window (some 40) none 7 [4, 6, 7],
  window none (some 20) 7 [0, 1],
  window (some 20) (some 40) 4 [2, 3],
  window (some 25) (some 26) 7 [5],
  window (some 61) none 7 []
]

/-- 模型对表里每一项给出那一项写下的 seq。 -/
theorem scenes_agree :
    scenes.all (fun c => (selected c.selection c.cutoff history).map (·.seq) == c.seqs) = true := by
  decide

/-- 表里没有矛盾的区间：Rust 对每一项都输出 bundle，而不是拒绝。 -/
theorem scenes_are_not_contradictory :
    scenes.all (fun c => c.selection.first.all (fun a => c.selection.last.all (a ≤ ·)) &&
      c.selection.since.all (fun a => c.selection.before.all (a < ·))) = true := by
  decide

end Accounting.Playback.Select
