-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::turn 的模型

证明 `turn`、`turn::boundary`、`turn::report`、`turn::wave`、`turn::wave::reorder`、`turn::ledger`、`turn::prompt`（`crates/runtime/src/` 下同名的文件）必须守住的性质。回合的 typestate 四相、四个取消点、工具波的入账次序与回合的账本门。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Turn

/-- 一道边界上到达的信号（`turn::Interrupt`）。Steer 的来源与正文不影响写下哪一行，所以模型不带它们。 -/
inductive Interrupt where
  | None
  | Cancel
  | Steer
  deriving DecidableEq, Repr

/-- 一个回合结束的方式（`turn::PhaseOutcome` 的两臂）。 -/
inductive Ending where
  | Advanced
  | Cancelled
  deriving DecidableEq, Repr

/-- 一个回合写进账本的行，以 `kernel::EventKind` 的名字命名；`prompt_assembled` 每个 run 只写一次（§8-39），不在这里。 -/
inductive Line where
  | PromptShapeCompared
  | ModelCalled
  | ModelReturned
  | ToolCalled (call : Nat)
  | ToolResult (call : Nat)
  | SteerReceived
  | CancelReceived
  deriving DecidableEq, Repr

/-- 哪几种行的 `t` 是为它自己采的读数，其余的带回合时间戳（`turn::ledger` 的 `Authored`／`Carried` 变体）。 -/
def measured : Line → Bool
  | .ModelCalled => true
  | .ModelReturned => true
  | .ToolCalled _ => true
  | .ToolResult _ => true
  | .PromptShapeCompared => false
  | .SteerReceived => false
  | .CancelReceived => false

/-- `consume_boundary`：一道边界消费一个信号，写下什么、回合还走不走。Steer 记一行照常前进；Cancel 记一行，回合到此为止。 -/
def consume_boundary : Interrupt → List Line × Bool
  | .None => ([], true)
  | .Steer => ([.SteerReceived], true)
  | .Cancel => ([.CancelReceived], false)

/-- 第 `first` 条起的 `count` 条调用各自的两行，按调用序。 -/
def accounted (first : Nat) : Nat → List Line
  | 0 => []
  | count + 1 => [.ToolCalled first, .ToolResult first] ++ accounted (first + 1) count

/-- 工具波：第 `call` 条调用之前问 `still_going call`，答案交给同一个 `consume_boundary`；走下去就按调用序写它的 `tool_called` 与 `tool_result`。开头只读段并行执行时次序与载荷不变，由 `spec/Turn/Speculation.lean` 证明；这里只管边界。 -/
def wave (still_going : Nat → Interrupt) (call : Nat) : Nat → List Line × Bool
  | 0 => ([], true)
  | remaining + 1 =>
    let boundary := consume_boundary (still_going call)
    if boundary.2 then
      let rest := wave still_going (call + 1) remaining
      (boundary.1 ++ [.ToolCalled call, .ToolResult call] ++ rest.1, rest.2)
    else (boundary.1, false)

/-- `wave` 走一步，写成一条等式，好让证明一次只展开最外一层。 -/
theorem wave_succ (still_going : Nat → Interrupt) (call remaining : Nat) :
    wave still_going call (remaining + 1) =
      if (consume_boundary (still_going call)).2 then
        ((consume_boundary (still_going call)).1 ++ [.ToolCalled call, .ToolResult call] ++
            (wave still_going (call + 1) remaining).1,
          (wave still_going (call + 1) remaining).2)
      else ((consume_boundary (still_going call)).1, false) := rfl

/-- 一个回合在四个取消点上收到的信号，以及波里每条调用之前的那一问。 -/
structure Signals where
  before_assemble : Interrupt
  before_call : Interrupt
  before_wave : Interrupt
  still_going : Nat → Interrupt
  before_spawn : Interrupt

/-- 回合的四相：组装、调用、工具波、收尾。每一相先消费它的边界，命中 Cancel 就写下 `cancel_received` 并结束；否则做这一相的事。`calls` 是模型这一答发出的调用数。 -/
def turn (signals : Signals) (calls : Nat) : List Line × Ending :=
  let assemble := consume_boundary signals.before_assemble
  if !assemble.2 then (assemble.1, .Cancelled) else
  let call := consume_boundary signals.before_call
  let called := assemble.1 ++ [.PromptShapeCompared] ++ call.1
  if !call.2 then (called, .Cancelled) else
  let opening := consume_boundary signals.before_wave
  let returned := called ++ [.ModelCalled, .ModelReturned] ++ opening.1
  if !opening.2 then (returned, .Cancelled) else
  let landed := wave signals.still_going 0 calls
  let waved := returned ++ landed.1
  if !landed.2 then (waved, .Cancelled) else
  let spawn := consume_boundary signals.before_spawn
  (waved ++ spawn.1, if spawn.2 then .Advanced else .Cancelled)

/-- 没有人喊停的一波，按调用序给每条调用记下两行。 -/
theorem an_uninterrupted_wave_accounts_every_call (still_going : Nat → Interrupt)
    (quiet : ∀ call, still_going call = .None) :
    ∀ (count first : Nat), wave still_going first count = (accounted first count, true)
  | 0, _ => rfl
  | count + 1, first => by
    rw [wave_succ, an_uninterrupted_wave_accounts_every_call still_going quiet count (first + 1)]
    simp [consume_boundary, quiet, accounted]

/-- **取消落在第 k 条调用之前，账本上恰好是前 k 条调用的两行，再一条 `cancel_received`。** 串行的波在同一处停下之前做的就是这几条，后面的调用一条也不写，所以账本上没有一条没有结果的调用。 -/
theorem a_cancel_before_a_call_accounts_exactly_the_calls_before_it
    (still_going : Nat → Interrupt) :
    ∀ (before first remaining : Nat),
      (∀ j, j < before → still_going (first + j) = .None) →
      still_going (first + before) = .Cancel →
      wave still_going first (before + remaining + 1) =
        (accounted first before ++ [.CancelReceived], false)
  | 0, first, remaining, _, stopped => by
    have stopsHere : still_going first = .Cancel := by simpa using stopped
    rw [show 0 + remaining + 1 = remaining + 1 by omega, wave_succ]
    simp [accounted, consume_boundary, stopsHere]
  | before + 1, first, remaining, quiet, stopped => by
    have here : still_going first = .None := by
      simpa using quiet 0 (Nat.succ_pos before)
    have later : ∀ j, j < before → still_going (first + 1 + j) = .None := by
      intro j below
      have shifted := quiet (j + 1) (by omega)
      rwa [show first + (j + 1) = first + 1 + j by omega] at shifted
    have stoppedLater : still_going (first + 1 + before) = .Cancel := by
      rwa [show first + (before + 1) = first + 1 + before by omega] at stopped
    have rest := a_cancel_before_a_call_accounts_exactly_the_calls_before_it still_going
      before (first + 1) remaining later stoppedLater
    rw [show before + 1 + remaining + 1 = (before + remaining + 1) + 1 by omega, wave_succ, rest]
    simp [accounted, consume_boundary, here]

/-- 一道边界上的 Steer 不结束回合：只要四个取消点与波里每一问都不是 Cancel，回合就前进。 -/
theorem a_steer_never_ends_a_turn (signals : Signals) (calls : Nat)
    (assemble : signals.before_assemble ≠ .Cancel) (call : signals.before_call ≠ .Cancel)
    (opening : signals.before_wave ≠ .Cancel) (spawn : signals.before_spawn ≠ .Cancel)
    (going : ∀ k, signals.still_going k ≠ .Cancel) :
    (turn signals calls).2 = .Advanced := by
  have keeps : ∀ signal : Interrupt, signal ≠ .Cancel → (consume_boundary signal).2 = true := by
    intro signal notCancel
    cases signal with
    | None => rfl
    | Steer => rfl
    | Cancel => exact absurd rfl notCancel
  have waves : ∀ (count first : Nat), (wave signals.still_going first count).2 = true := by
    intro count
    induction count with
    | zero => intro _; rfl
    | succ count ih =>
      intro first
      rw [wave_succ]
      simp [keeps _ (going first), ih]
  simp [turn, keeps _ assemble, keeps _ call, keeps _ opening, keeps _ spawn, waves]

/-- **被取消的波以它的 `cancel_received` 结束，之后什么都不写**：回合后面的 `handoff_written` 与 `run_frozen` 归 `run` 的 `freeze`（§8-15）。 -/
theorem a_cancelled_wave_ends_with_its_cancel (still_going : Nat → Interrupt) :
    ∀ (count first : Nat), (wave still_going first count).2 = false →
      ∃ before, (wave still_going first count).1 = before ++ [.CancelReceived]
  | 0, _, ended => by simp [wave] at ended
  | count + 1, first, ended => by
    rw [wave_succ] at ended
    rw [wave_succ]
    cases signal : still_going first with
    | Cancel => exact ⟨[], by simp [consume_boundary]⟩
    | None =>
      have restEnded : (wave still_going (first + 1) count).2 = false := by
        simpa [consume_boundary, signal] using ended
      obtain ⟨before, ends⟩ :=
        a_cancelled_wave_ends_with_its_cancel still_going count (first + 1) restEnded
      exact ⟨[.ToolCalled first, .ToolResult first] ++ before, by simp [consume_boundary, ends]⟩
    | Steer =>
      have restEnded : (wave still_going (first + 1) count).2 = false := by
        simpa [consume_boundary, signal] using ended
      obtain ⟨before, ends⟩ :=
        a_cancelled_wave_ends_with_its_cancel still_going count (first + 1) restEnded
      exact ⟨[.SteerReceived, .ToolCalled first, .ToolResult first] ++ before,
        by simp [consume_boundary, ends]⟩

end Runtime.Turn
