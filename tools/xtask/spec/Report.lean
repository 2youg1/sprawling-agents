-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 一次 `gates` 运行的结论：按门序汇合，退出码取最重的一态

规定 `tools/xtask/src/report.rs` 的 `finish_all` 与 `RunOutcome`，以及 `tools/xtask/src/gates.rs`
把每道门的结论按门序交给它的那一步（`tools/xtask/Spec.lean` §8-31）。

参照定义只陈述汇合规则：每道门交出「判过，带若干条违规」或「判不动」，一次运行走完全部结论，
数出判不动的门与全部违规，再取一态。门怎样判、违规怎样渲染不在这里，那一半在各门自己的 Rust
里，由它们自己的测试守着。
-/

namespace Xtask.Report

/-- 一道门的结论：判过，带 `findings` 条违规；或判不动（`XtaskError`，退出码 2）。 -/
inductive Judged where
  | judged (findings : Nat)
  | couldNotJudge
deriving DecidableEq, Repr

/-- 一次运行的结论，按严重度排：门坏了（2）压过树坏了（1），树坏了压过全绿（0）。 -/
inductive RunOutcome where
  | Green
  | Refused
  | Broken
deriving DecidableEq, Repr

/-- 每一态的退出码。 -/
def RunOutcome.code : RunOutcome → Nat
  | .Green => 0
  | .Refused => 1
  | .Broken => 2

/-
D2 **一门判不动，不得连累其余各门的结论。**`gates` 的那张数组是急切求值的，
<!-- xtask:begin gate_count -->24<!-- xtask:end --> 道门在第一行输出之前就已全部跑完；聚合运行遍历到底，
逐门打印 `gate <name>: ok`、违规数或 `gate <name>: could not judge`，再统一渲染全部违规
（`report.rs`）。**退出码取最重的一态**：任一门判不动为 2，否则有违规为 1，否则 0——判不动压过
判有罪，因为「没判」与「判过且干净」同形正是本条要拆开的东西。理由：缺某道门要用的
可选工具是 `docs/CONTRIBUTING.md` 明列的预期状态，在那种机器上，被判不动的门不能吞掉排在它后面、
已判出结论的门。被击败的备选：遇到第一个 `Err` 即返回，那样一次带违规的运行与一次干净的运行
输出可以逐字相同。
-/

/-- 由判不动的门数与违规总数取一态（`RunOutcome::of`）。 -/
def RunOutcome.of (broken findings : Nat) : RunOutcome :=
  if broken > 0 then .Broken
  else if findings > 0 then .Refused
  else .Green

/-- 一串结论里判不动的门数。 -/
def broken : List Judged → Nat
  | [] => 0
  | .couldNotJudge :: rest => broken rest + 1
  | .judged _ :: rest => broken rest

/-- 一串结论里判出的违规总数；判不动的门不带违规。 -/
def findings : List Judged → Nat
  | [] => 0
  | .couldNotJudge :: rest => findings rest
  | .judged count :: rest => count + findings rest

/-- 一次运行走完全部结论之后的那一态（`finish_all`）。 -/
def finish_all (results : List Judged) : RunOutcome :=
  RunOutcome.of (broken results) (findings results)

theorem broken_append (front back : List Judged) :
    broken (front ++ back) = broken front + broken back := by
  induction front with
  | nil => simp [broken]
  | cons head rest ih =>
    cases head <;> simp [broken, ih] <;> omega

theorem findings_append (front back : List Judged) :
    findings (front ++ back) = findings front + findings back := by
  induction front with
  | nil => simp [findings]
  | cons head rest ih =>
    cases head <;> simp [findings, ih] <;> omega

/-- 判不动的门不吞掉排在它后面的结论：它前后各门判出的违规一条不少地数进这一次运行。 -/
theorem a_broken_gate_drops_no_later_finding (front back : List Judged) :
    findings (front ++ .couldNotJudge :: back) = findings front + findings back := by
  simp [findings_append, findings]

/-- 只要有一道门判不动，这次运行就是 `Broken`，不论别的门判出什么。 -/
theorem a_broken_gate_outranks_a_violation (front back : List Judged) :
    finish_all (front ++ .couldNotJudge :: back) = .Broken := by
  simp [finish_all, RunOutcome.of, broken_append, broken]

/-- 全绿恰是每道门都判过、且一条违规都没判出：「没判」读不成「干净」。 -/
theorem green_means_every_gate_judged_and_found_nothing (results : List Judged) :
    finish_all results = .Green ↔ broken results = 0 ∧ findings results = 0 := by
  unfold finish_all RunOutcome.of
  by_cases b : broken results > 0 <;> by_cases f : findings results > 0 <;> simp [b, f] <;> omega

theorem broken_perm {one other : List Judged} (same : one.Perm other) :
    broken one = broken other := by
  induction same with
  | nil => rfl
  | cons head _ ih => cases head <;> simp [broken, ih]
  | swap x y rest => cases x <;> cases y <;> simp [broken]
  | trans _ _ ih₁ ih₂ => exact ih₁.trans ih₂

theorem findings_perm {one other : List Judged} (same : one.Perm other) :
    findings one = findings other := by
  induction same with
  | nil => rfl
  | cons head _ ih => cases head <;> simp [findings, ih]
  | swap x y rest => cases x <;> cases y <;> simp [findings] <;> omega
  | trans _ _ ih₁ ih₂ => exact ih₁.trans ih₂

/-- 门序换了，结论不变：退出码只取决于数出来的两个数，所以并行判定、按门序报告不改变它。 -/
theorem the_outcome_ignores_gate_order (one other : List Judged) (same : one.Perm other) :
    finish_all one = finish_all other := by
  simp [finish_all, broken_perm same, findings_perm same]

/-- 三态都到得了：一次干净的运行是绿的，一次判出违规的运行被拒，规则不是空真。 -/
theorem every_outcome_is_reachable :
    finish_all [.judged 0, .judged 0] = .Green ∧
    finish_all [.judged 0, .judged 3] = .Refused ∧
    finish_all [.judged 3, .couldNotJudge] = .Broken := by
  decide

end Xtask.Report
