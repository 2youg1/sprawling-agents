-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::run 的模型

证明 `run`、`run::lifecycle`、`run::charter`、`run::harness`（`crates/runtime/src/` 下同名的文件）必须守住的性质。run 的驱动：派发、回合、冻结，run 事件序与 harness run 事件序的唯一权威。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Run

/-- 一次 run 冻成什么（`kernel::Completion`）。`Done` 带的证据不影响下面的性质，模型不带它。 -/
inductive Completion where
  | Done
  | Limit
  | Cancelled
  deriving DecidableEq, Repr

/-- `lifecycle::concluded`：一答没有调用时冻成什么。说了话、又不是撞上输出上限停下的，是 `Done`；什么都没说的，或停在 `max_tokens` 上的，是 `Limit`（§8-37）。 -/
def concluded (said : Bool) (stoppedAtMaxTokens : Bool) : Completion :=
  if !said || stoppedAtMaxTokens then .Limit else .Done

/-- 一条空回复不是「做完了」，撞上输出上限也不是。 -/
theorem done_needs_words_and_no_ceiling (said stoppedAtMaxTokens : Bool) :
    concluded said stoppedAtMaxTokens = .Done ↔ said = true ∧ stoppedAtMaxTokens = false := by
  cases said <;> cases stoppedAtMaxTokens <;> decide

/-- `run::drive` 循环里一次 `advance` 的结局，以及它的失败分哪两类。 -/
inductive Step where
  /-- 一个回合走完，还有调用：再来一回合。 -/
  | Turned
  /-- 一个回合走完，这一答没有调用，或某个安全点命中了 Cancel。 -/
  | Concluded (completion : Completion)
  /-- 可以再问的 provider 失败：先写 `watchdog_fired`，等到 `until`，再来。 -/
  | Retried
  /-- 回合中途不能再问的失败：冻成 `Cancelled`，原错误随后向上抛。 -/
  | Failed
  deriving DecidableEq, Repr

/-- run 写下的行，按 run 的事件序关心的样子：回合内的各行是一个 `Turn`，它们的次序归 `spec/Turn.lean`。 -/
inductive Line where
  | JobPinned
  | RunStarted
  | Turn
  | WatchdogFired
  | HandoffWritten
  | RunFrozen
  deriving DecidableEq, Repr

/-- `drive` 的循环：一路写到第一个结局，结局经 `freeze` 写下收尾两行。剧本里的步数用完而还没有结局时，run 仍在跑，第二个分量是 `none`。 -/
def loop : List Step → List Line × Option Completion
  | [] => ([], none)
  | .Turned :: rest => ((.Turn :: (loop rest).1), (loop rest).2)
  | .Retried :: rest => ((.WatchdogFired :: (loop rest).1), (loop rest).2)
  | .Concluded completion :: _ => ([.Turn, .HandoffWritten, .RunFrozen], some completion)
  | .Failed :: _ => ([.HandoffWritten, .RunFrozen], some .Cancelled)

/-- `run::drive`：开篇两行（`Charter::open`），然后循环。 -/
def drive (steps : List Step) : List Line × Option Completion :=
  ([.JobPinned, .RunStarted] ++ (loop steps).1, (loop steps).2)

/-- **`handoff_written`＋`run_frozen` 是唯一出口。** 一次 run 不论以什么结局结束，账本都以这两行收尾，而 `run_frozen` 只写一次。 -/
theorem freeze_is_the_only_exit :
    ∀ (steps : List Step), (loop steps).2.isSome = true →
      ∃ before, (loop steps).1 = before ++ [.HandoffWritten, .RunFrozen] ∧ .RunFrozen ∉ before
  | [], ended => by simp [loop] at ended
  | .Turned :: rest, ended => by
    obtain ⟨before, ends, once⟩ := freeze_is_the_only_exit rest (by simpa [loop] using ended)
    exact ⟨.Turn :: before, by simp [loop, ends], by simp [once]⟩
  | .Retried :: rest, ended => by
    obtain ⟨before, ends, once⟩ := freeze_is_the_only_exit rest (by simpa [loop] using ended)
    exact ⟨.WatchdogFired :: before, by simp [loop, ends], by simp [once]⟩
  | .Concluded _ :: _, _ => ⟨[.Turn], rfl, by decide⟩
  | .Failed :: _, _ => ⟨[], rfl, by decide⟩

/-- 开篇两行恒在最前：`JobPinned` 在 `run_started` 之前。 -/
theorem the_job_is_pinned_before_the_run_starts (steps : List Step) :
    (drive steps).1.take 2 = [.JobPinned, .RunStarted] := by
  simp [drive]

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `TimeMs` 能装下的最大一刻。 -/
def TIME_MAX : Nat := 2 ^ 64 - 1

/-- `Charter::close`：`handoff_written` 记 `t`，`run_frozen` 记 `t + 1`，两行记的是同一件事；`t` 已到顶时拒绝，而不是回绕。 -/
def close (t : Nat) : Except Code (Nat × Nat) :=
  if t < TIME_MAX then .ok (t, t + 1) else .error .InvalidArgs

/-- 收尾两行相隔一毫秒，`run_frozen` 在后。 -/
theorem run_frozen_follows_its_handoff_by_one_millisecond (t handoff frozen : Nat)
    (closed : close t = .ok (handoff, frozen)) : handoff = t ∧ frozen = handoff + 1 := by
  unfold close at closed
  split at closed
  · injection closed with same
    injection same with first second
    subst first
    subst second
    exact ⟨rfl, rfl⟩
  · cases closed

/-- 时钟到顶时冻结拒绝，`run_frozen` 不会记成比 `handoff_written` 更早的一刻。 -/
theorem a_clock_at_its_ceiling_cannot_close : close TIME_MAX = .error .InvalidArgs := by
  simp [close]

end Runtime.Run
