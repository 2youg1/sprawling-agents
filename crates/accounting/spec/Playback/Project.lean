-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# playback 的投影：读者读不到的行不流进任何派生表，关键时刻只在真实结束事件上闭合

规定 `crates/accounting/src/playback/project.rs`、`crates/accounting/src/playback/reader.rs`、`crates/accounting/src/playback/links.rs`、`crates/accounting/src/playback/traced.rs` 与 `crates/accounting/src/trace.rs`（`crates/accounting/Spec.lean` §8-12、§8-25）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

**读界。** 一行碰到的每一栋楼都由读者判一次，答案复用 `kernel::ReadVerdict` 的三臂：`Open`、`Confidential`、`RulesUnreadable`。只有碰到的楼全是 `Open` 的行可见；另外两臂一律关闭，所以规则读不了不会朝宽的一侧失败。一行「碰到」哪些楼，Rust 从信封地址、它所在 run 的房间、它关闭的那一对的打开行、以及载荷里以已知楼开头的地址求出；本模型把这个集合当作行的一个字段。

**凭据扫描不在模型里。** Rust 里碰到的楼全是 `Open` 的行，还要过 `kernel::secret::scan`：扫出凭据的行同样隐去，只记一条 `credential` 计数（`playback::project` 的 `Fate::Credential`）。这一条读的是行的内容，而本模型的 `readable` 只读 `touches`，所以 `hidden_content_never_reaches_the_bundle` 说的是「关闭的楼」这一半；凭据那一半只由 `playback::project` 里那一处判定持有，「什么像凭据」的权威是 `kernel::secret::scan`。

**派生表只读可见行。** 事件表、run 表、关键时刻、消息、费用与 checkpoint 表都是 `derive` 在可见行上的值；被隐去的行只留下条数（以及 Rust 里列明的楼名与种类计数，这些是明示的元数据披露）。`hidden_content_never_reaches_the_bundle` 陈述的是：把被隐去的行改成任何别的内容（只要它仍被隐去、seq 不变），bundle 一字不变——所以被隐去的内容没有任何一条路流进 bundle。

**真实关闭的单调性。** 一个关键时刻（一个 run、一次审批、一个 PR、一封信）在 cutoff `c` 闭合，当且仅当某一行 seq ≤ c 且它是这个键的关闭事件。闭合只看关闭事件，不看选择窗口的右端：`closedAt` 的参数里没有窗口，这是定义的形状，不另立定理；在 cutoff c 闭合的键，在更晚的 cutoff、在追加了行的账本上仍闭合（`closure_is_monotone`、`closure_survives_appends`）。前缀扩展只对这些已真实闭合的对象成立：还开着的对象在更晚的 cutoff 可能闭合，那是变化而不是矛盾。

**调用的耗时。** 工具调用与模型调用的 `took` 是同一条规则：两端的时刻都量过、答复不是城补写的、答复不早于调用，才给出两者之差；其余一律是 unknown，从不是零（`took_is_measured`、`an_unmeasured_end_is_unknown`）。一条答复配哪一次调用是 Rust 的配对规则（工具调用按 id，模型调用按 `views::rounds::Attempts`），本模型把配好的两端当作输入。

**提交的证据只读到它的宣告行。** 一个提交的证据是宣告它的那一行 `s` 与之前的行折出的值；`s` 在 cutoff 以内时，cutoff 之后追加的行，不论内容是什么、能不能通过审计，都改变不了它（`evidence_ignores_lines_after_the_cutoff`）。Rust 里 `trace::History` 逐行折到这一行时问视图，`trace::trace_through` 只读 seq 小于这一行的行。

本模型不是 Rust 实现的证明；两者的一致由 `accounting::playback::tests` 在同一组场景上的比较守住。
-/

namespace Accounting.Playback.Project

/-- `kernel::ReadVerdict` 的三臂。 -/
inductive Verdict where
  | open
  | confidential
  | rulesUnreadable
  deriving Repr, DecidableEq

/-- 一行：seq、它碰到的楼、它关闭的关键时刻键（若有）、以及内容（抽象成一个数）。 -/
structure Line where
  seq : Nat
  touches : List Nat
  closes : Option Nat
  content : Nat
  deriving Repr, DecidableEq

/-- 读者对一行的判断：碰到的每一栋楼都 `Open` 才可见。 -/
def readable (verdict : Nat → Verdict) (l : Line) : Bool :=
  l.touches.all (fun b => verdict b == .open)

def visible (verdict : Nat → Verdict) (lines : List Line) : List Line :=
  lines.filter (readable verdict)

def hiddenCount (verdict : Nat → Verdict) (lines : List Line) : Nat :=
  (lines.filter (fun l => !readable verdict l)).length

/-- bundle：可见行、它们的派生表、被隐去的条数。 -/
structure Bundle (α : Type) where
  events : List Line
  derived : α
  withheld : Nat

def bundle {α : Type} (derive : List Line → α) (verdict : Nat → Verdict) (lines : List Line) :
    Bundle α :=
  let seen := visible verdict lines
  { events := seen, derived := derive seen, withheld := hiddenCount verdict lines }

/-! ## 读者读不到的行不流进派生表 -/

theorem every_event_is_readable {α : Type} (derive : List Line → α) (verdict : Nat → Verdict)
    (lines : List Line) : ∀ l ∈ (bundle derive verdict lines).events, readable verdict l = true := by
  intro l hl
  simp [bundle, visible, List.mem_filter] at hl
  exact hl.2

/-- 一次改写只动被隐去的行，并让它仍被隐去。 -/
def TouchesOnlyHidden (verdict : Nat → Verdict) (g : Line → Line) : Prop :=
  ∀ l, (readable verdict l = true → g l = l) ∧
    (readable verdict l = false → readable verdict (g l) = false)

theorem visible_ignores_hidden (verdict : Nat → Verdict) (g : Line → Line)
    (hg : TouchesOnlyHidden verdict g) (lines : List Line) :
    visible verdict (lines.map g) = visible verdict lines := by
  induction lines with
  | nil => rfl
  | cons l rest ih =>
    cases hr : readable verdict l with
    | true =>
      have hl : g l = l := (hg l).1 hr
      simp [visible, hl, hr] at ih ⊢
      exact ih
    | false =>
      have hgl : readable verdict (g l) = false := (hg l).2 hr
      simp [visible, hgl, hr] at ih ⊢
      exact ih

theorem hidden_count_ignores_hidden (verdict : Nat → Verdict) (g : Line → Line)
    (hg : TouchesOnlyHidden verdict g) (lines : List Line) :
    hiddenCount verdict (lines.map g) = hiddenCount verdict lines := by
  induction lines with
  | nil => rfl
  | cons l rest ih =>
    cases hr : readable verdict l with
    | true =>
      have hl : g l = l := (hg l).1 hr
      simp [hiddenCount, hl, hr] at ih ⊢
      exact ih
    | false =>
      have hgl : readable verdict (g l) = false := (hg l).2 hr
      simp [hiddenCount, hgl, hr] at ih ⊢
      exact ih

theorem hidden_content_never_reaches_the_bundle {α : Type} (derive : List Line → α)
    (verdict : Nat → Verdict) (g : Line → Line) (hg : TouchesOnlyHidden verdict g)
    (lines : List Line) :
    bundle derive verdict (lines.map g) = bundle derive verdict lines := by
  simp only [bundle, visible_ignores_hidden verdict g hg, hidden_count_ignores_hidden verdict g hg]

/-- 实现可达：一栋机密楼的行被隐去，公开楼的行留下，隐去条数是 1。 -/
theorem a_confidential_line_is_counted_not_shown :
    let verdict : Nat → Verdict := fun b => if b = 1 then .confidential else .open
    let lines := [⟨0, [0], none, 7⟩, ⟨1, [0, 1], none, 8⟩, ⟨2, [], none, 9⟩]
    (bundle (fun seen => seen.length) verdict lines).withheld = 1 ∧
      (bundle (fun seen => seen.length) verdict lines).derived = 2 := by
  decide

/-! ## 真实关闭的单调性 -/

/-- 键 `key` 在 cutoff `c` 已闭合：某一行 seq ≤ c 且是它的关闭事件。 -/
def closedAt (key c : Nat) (lines : List Line) : Bool :=
  lines.any (fun l => decide (l.seq ≤ c) && l.closes == some key)

theorem closure_is_monotone (key c c' : Nat) (lines : List Line) (hle : c ≤ c')
    (h : closedAt key c lines = true) : closedAt key c' lines = true := by
  simp only [closedAt, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h ⊢
  obtain ⟨l, hl, hs, hk⟩ := h
  exact ⟨l, hl, Nat.le_trans hs hle, hk⟩

theorem closure_survives_appends (key c : Nat) (lines more : List Line)
    (h : closedAt key c lines = true) : closedAt key c (lines ++ more) = true := by
  simp only [closedAt] at h ⊢
  rw [List.any_append, h, Bool.true_or]

/-! 窗口的右端不是关闭事件：一个在窗口内打开、到 cutoff 仍没有关闭行的键没有闭合，不论窗口在哪里结束。`closedAt` 不读选择，它的参数里没有窗口。 -/

/-! ## 调用的耗时：量出来的才给，其余是 unknown -/

/-- 配好的一次调用：调用行与答复行各自的时刻（`EventRecord::moment`，没量过或那一端不可见为 `none`），以及答复是不是城在重启后补写的（`E_TOOL_OUTCOME_UNKNOWN`；模型的答复从不是）。 -/
structure Timed where
  called : Option Nat
  answered : Option Nat
  supplied : Bool
  deriving Repr, DecidableEq

/-- `took`：两端都量过、答复不是补写、答复不早于调用时是两者之差（Rust 的 `{"measured": …}`），其余是 `none`（Rust 的 `"unknown"`）。 -/
def took (c : Timed) : Option Nat :=
  match c.called, c.answered with
  | some a, some b => if c.supplied = false ∧ a ≤ b then some (b - a) else none
  | _, _ => none

/-- 给出的耗时都是量出来的：两端都有时刻，答复不是补写，答复不早于调用，值是两者之差。 -/
theorem took_is_measured (c : Timed) (d : Nat) (h : took c = some d) :
    ∃ a b, c.called = some a ∧ c.answered = some b ∧ c.supplied = false ∧ a ≤ b ∧ d = b - a := by
  obtain ⟨called, answered, supplied⟩ := c
  cases called with
  | none => simp [took] at h
  | some a =>
    cases answered with
    | none => simp [took] at h
    | some b =>
      simp only [took] at h
      split at h
      · rename_i hold
        exact ⟨a, b, rfl, rfl, hold.1, hold.2, (Option.some.inj h).symm⟩
      · exact absurd h (by simp)

/-- 少一端的时刻就没有耗时：旧版本的行、没有答复、一端被隐去，都不会被画成零。 -/
theorem an_unmeasured_end_is_unknown (c : Timed) (h : c.called = none ∨ c.answered = none) :
    took c = none := by
  obtain ⟨called, answered, supplied⟩ := c
  rcases h with h | h
  · simp only at h
    subst h
    cases answered <;> rfl
  · simp only at h
    subst h
    cases called <;> rfl

/-- 实现可达：量过的一对给出差值，补写的答复与没有答复的调用都是 unknown。 -/
theorem a_measured_call_is_timed_and_the_rest_are_not :
    took ⟨some 2000, some 2350, false⟩ = some 350 ∧ took ⟨some 3000, some 9000, true⟩ = none ∧
      took ⟨some 9500, none, false⟩ = none := by
  decide

/-! ## 提交的证据只读到它的宣告行 -/

/-- 一个提交的证据：宣告它的那一行 `s` 与之前的行折出的值。 -/
def evidenceAt {α : Type} (fold : List Line → α) (s : Nat) (lines : List Line) : α :=
  fold (lines.filter (fun l => decide (l.seq ≤ s)))

/-- 宣告行在 cutoff 以内时，cutoff 之后追加的行改变不了证据，所以复核时这个提交不会因为后来的历史变成 `unread`。 -/
theorem evidence_ignores_lines_after_the_cutoff {α : Type} (fold : List Line → α) (s c : Nat)
    (lines more : List Line) (hs : s ≤ c) (later : ∀ l ∈ more, c < l.seq) :
    evidenceAt fold s (lines ++ more) = evidenceAt fold s lines := by
  have none : more.filter (fun l => decide (l.seq ≤ s)) = [] := by
    rw [List.filter_eq_nil_iff]
    intro l hl
    have := later l hl
    simp only [decide_eq_true_eq]
    omega
  simp [evidenceAt, List.filter_append, none]

end Accounting.Playback.Project
