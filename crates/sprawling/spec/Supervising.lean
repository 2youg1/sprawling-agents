-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 守护的崩溃预算：崩溃 → `resume` → `serve`，一分钟三次就停下等人

规定 `crates/sprawling/src/supervising.rs` 的 `CrashBudget::after`（`bin::supervising`，形状 1 decision；sprawling-SPEC.md 8-109）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。起子进程、读退出状态、等人按 Enter 归 `bin::supervising::children`，这里不建模。

时刻是守护起步以来的毫秒数，取自单调时钟，所以在模型里是自然数；「距今」是自然数的截断减法，与 Rust 的 `saturating_sub` 相同。

五条性质：

* 人选择的收口（`Chosen`）总是停下；
* 一次重启带出的预算少于 `limit` 次崩溃、每一次都还在窗口里、最后一次就是这一次——预算占的内存有界；
* 窗口里凑满三次即 `degraded`，不再重启；
* 崩溃之间隔满一个窗口时永远重启，预算里只有这一次；
* 退出码 2 也是崩溃（D2）：`Closing` 只分人选的与坏掉的两种，没有第三种「不必重启」。
-/

namespace Sprawling.Supervising

/-- 窗口与次数（Rust：`CRASH_WINDOW_MS`、`CRASH_LIMIT`）。 -/
def crashWindowMs : Nat := 60000
def crashLimit : Nat := 3

/-- 子进程怎么收口（Rust：`accounting::worker::Closing`，由 `Closing::of` 读退出状态：0 为 `chosen`，其余一切为 `broken`）。 -/
inductive Closing where
  | chosen
  | broken (cause : String)
  deriving Repr, DecidableEq

/-- 窗口里的崩溃时刻，从旧到新（Rust：`CrashBudget`）。 -/
abbrev CrashBudget := List Nat

/-- 下一步（Rust：`Next`）。 -/
inductive Next where
  | stop
  | restart (budget : CrashBudget)
  | degraded (crashes : Nat) (cause : String)
  deriving Repr, DecidableEq

/-- 还在窗口里的旧崩溃，再加上这一次。 -/
def recent (budget : CrashBudget) (at_ : Nat) : CrashBudget :=
  budget.filter (fun c => decide (at_ - c < crashWindowMs)) ++ [at_]

/-- `CrashBudget::after`。 -/
def after (budget : CrashBudget) (closing : Closing) (at_ : Nat) : Next :=
  match closing with
  | .chosen => .stop
  | .broken cause =>
    if (recent budget at_).length ≥ crashLimit then .degraded (recent budget at_).length cause
    else .restart (recent budget at_)

theorem chosen_stops (budget : CrashBudget) (at_ : Nat) : after budget .chosen at_ = .stop := rfl

/-- 一次重启带出的预算：少于 `crashLimit` 次，每一次都在窗口里，最后一次就是这一次。 -/
theorem restart_is_bounded (budget kept : CrashBudget) (cause : String) (at_ : Nat)
    (h : after budget (.broken cause) at_ = .restart kept) :
    kept.length < crashLimit ∧ (∀ c ∈ kept, at_ - c < crashWindowMs) ∧ kept.getLast? = some at_ := by
  have hl : ¬ (recent budget at_).length ≥ crashLimit := by
    intro hl; simp [after, hl] at h
  have hk : kept = recent budget at_ := by
    simp [after, hl] at h; exact h.symm
  subst hk
  refine ⟨by omega, ?_, by simp [recent]⟩
  intro c hc
  simp [recent, List.mem_filter] at hc
  rcases hc with ⟨_, hc⟩ | hc
  · exact hc
  · subst hc; simp [crashWindowMs]

/-- 一分钟之内第三次崩溃：停下等人。 -/
theorem third_crash_in_a_window_degrades (a b c : Nat) (cause : String)
    (hab : a ≤ b) (hbc : b ≤ c) (hw : c - a < crashWindowMs) :
    ∃ k₁ k₂, after [] (.broken cause) a = .restart k₁ ∧
      after k₁ (.broken cause) b = .restart k₂ ∧
      after k₂ (.broken cause) c = .degraded 3 cause := by
  have hb : b - a < crashWindowMs := by omega
  have hc : c - b < crashWindowMs := by omega
  refine ⟨[a], [a, b], ?_, ?_, ?_⟩
  · simp [after, recent, crashLimit]
  · simp [after, recent, crashLimit, hb]
  · simp [after, recent, crashLimit, hw, hc]

/-- 崩溃之间隔满一个窗口：每一次都重启，预算里只有这一次。 -/
theorem spaced_crashes_always_restart (last at_ : Nat) (cause : String)
    (h : last + crashWindowMs ≤ at_) :
    after [last] (.broken cause) at_ = .restart [at_] := by
  have : ¬ at_ - last < crashWindowMs := by omega
  simp [after, recent, crashLimit, this]

end Sprawling.Supervising
