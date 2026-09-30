-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# playback 的选择：哪些行进 bundle、按什么次序、读到哪一行为止

规定 `crates/accounting/src/playback/select.rs` 与 `crates/accounting/src/playback/walk.rs`（accounting-SPEC.md 8-12）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一次导出先固定 cutoff：严格校验（`storage::LineCheck`）走到的最后一个完整行。范围内事件恰好是「seq 不超过 cutoff」与选择条件的交集；选择条件是 seq 的闭区间 `[first, last]`、一个 run、一栋楼，给了几项就取几项的交集。

四条性质，各一组定理：

* **选择是交集，不多不少**（`mem_selected`）；
* **次序与去重**：LineCheck 让账本的 seq 严格递增（`Chained`），被选中的行仍严格递增，所以每个 seq 恰好出现一次（`selected_chained`、`selected_seqs_nodup`）；
* **截止线不读未来**：cutoff 之后追加的行改变不了这次选择（`cutoff_ignores_future`）；
* **矛盾的范围与合法的空选择是两件事**：`first > last` 由 Rust 拒绝（`Contradictory`），合法而为空的选择输出带范围信息的空 bundle（`empty_selection_is_legal`）。

模型边界：楼的判定在 Rust 里是 `Address::is_within`（按段边界的前缀），这里抽象成楼的编号相等，`lab` 与 `laboratory` 的区别由 `is_within` 自己的测试守住（kernel-SPEC 8-2）。本模型不是 Rust 实现的证明；两者的一致由 `accounting::playback::tests` 在同一组场景上的比较守住。
-/

namespace Accounting.Playback.Select

/-- 账本的一行，只留选择读的字段：seq、run、信封地址所在的楼（没有地址时为 `none`）。 -/
structure Line where
  seq : Nat
  run : Nat
  building : Option Nat
  deriving Repr, DecidableEq

/-- 一次选择：seq 闭区间的两端、一个 run、一栋楼；`none` 表示这一项不限。 -/
structure Selection where
  first : Option Nat
  last : Option Nat
  run : Option Nat
  building : Option Nat
  deriving Repr, DecidableEq

/-- 一行是否满足选择的每一项。没有地址的行不属于任何一栋楼。 -/
def Selection.admits (s : Selection) (l : Line) : Bool :=
  s.first.all (· ≤ l.seq) && s.last.all (l.seq ≤ ·) &&
    s.run.all (· == l.run) && s.building.all (fun b => l.building == some b)

/-- 一个区间的两端是否矛盾：Rust 以 `E_INVALID_ARGS` 拒绝它，不输出 bundle。 -/
def Selection.Contradictory (s : Selection) : Prop :=
  ∃ a b, s.first = some a ∧ s.last = some b ∧ b < a

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

/-! ## 合法的空选择 -/

/-- 一段城里没有的 run 选出空表，而区间不矛盾：Rust 输出空 bundle，范围信息照写。 -/
theorem empty_selection_is_legal :
    let s : Selection := ⟨some 1, some 3, some 9, none⟩
    selected s 5 [⟨0, 0, none⟩, ⟨1, 1, some 0⟩, ⟨2, 1, some 0⟩] = [] ∧ ¬ s.Contradictory := by
  refine ⟨by decide, ?_⟩
  rintro ⟨a, b, ha, hb, hlt⟩
  simp at ha hb
  omega

/-- 实现可达：一个真实的选择选中范围内、楼内的行，漏掉别的楼的行与 cutoff 之后的行。 -/
theorem a_selection_reaches_lines :
    selected ⟨some 1, none, none, some 0⟩ 3
      [⟨0, 0, none⟩, ⟨1, 1, some 0⟩, ⟨2, 2, some 1⟩, ⟨3, 1, some 0⟩, ⟨4, 1, some 0⟩] =
      [⟨1, 1, some 0⟩, ⟨3, 1, some 0⟩] := by
  decide

end Accounting.Playback.Select
