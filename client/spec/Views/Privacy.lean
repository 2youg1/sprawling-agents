-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy：隐私页上一项从按下到结果

规定 `client/src/views/setup/privacy/entry.ts` 的 `step`：隐私组（`#/setup/privacy`，§7L）里每一个控制各有一台这样的状态机，页面没有「全部应用」，也不预选任何一项。主机一侧的确认绑定（命令带页面显示的当前值作 `expected`，主机 fresh read 不符即拒绝）在 `crates/sprawling/spec/Privacy/Confirmation.lean`；机器作用域另经 Windows 的 UAC。

一项的走法：按下「应用」「恢复」或「核对」（只有页面算出可用的那个动作按得动）进入确认；确认框写出当前值 → 写入值、作用域、是否需要管理员、这一项容易被忽略的事与怎样撤销；取消回到原处，什么也不发；确认时铸一个 idem 并发送命令，进入发送中；链路没送出时回到可重试的「未发出」；主机按 idem 保留结果，页面只认自己铸的那个 idem：仍在进行就留在发送中，结束时进入已结束或被拒绝，任何别的 idem 的结果都不改这一项。结束或被拒绝之后可以再按。

性质（对模型允许的每一条事件序列成立）：

1. **取消从不发送**（`cancel_never_sends`）。
2. **只有在确认框里确认才发送，发的是确认框里那个动作**（`only_a_confirmation_sends`）。
3. **别人的结果不改这一项**（`a_foreign_outcome_changes_nothing`）：结果按 idem 绑定，不读全局的拒绝。
4. **发送中不会再发**（`no_second_send_in_flight`）。
5. **没送出的可以再按**（`an_unsent_action_can_be_pressed_again`）。
6. **一条事件序列发出的命令，个数不超过其中确认事件的个数，且每一条都带那次确认的 idem**（`every_send_is_a_confirmation`）。

`#eval` 打印的轨迹向量由 `client/src/views/setup/privacy/entry.test.ts` 逐条回放，模型与实现一旦分歧，那个测试变红。
-/

namespace Client.Views.Privacy

/-- 一项能做的三个动作。 -/
inductive Action where
  | apply
  | restore
  | reconcile
  deriving DecidableEq, Repr

/-- 主机按 idem 答回的结果，只分页面需要区分的三种。 -/
inductive Result where
  | running
  | done
  | refused
  deriving DecidableEq, Repr

/-- 一项所处的阶段；idem 以自然数代表。 -/
inductive Stage where
  | idle
  | confirming (action : Action)
  | sending (action : Action) (idem : Nat)
  | unsent (action : Action)
  | settled (idem : Nat)
  | refused (idem : Nat)
  deriving DecidableEq, Repr

/-- 页面送进状态机的事件。`confirm` 带这次铸的 idem；`lost` 是链路没送出那个 idem。 -/
inductive Event where
  | press (action : Action)
  | cancel
  | confirm (idem : Nat)
  | lost (idem : Nat)
  | outcome (idem : Nat) (result : Result)
  deriving DecidableEq, Repr

/-- 一条发出的命令：动作与它的 idem。 -/
structure Send where
  action : Action
  idem : Nat
  deriving DecidableEq, Repr

/-- 可以按下的阶段：没有确认框开着，也没有命令在途。 -/
def pressable : Stage → Bool
  | .idle | .unsent _ | .settled _ | .refused _ => true
  | .confirming _ | .sending _ _ => false

/-- 一步：新阶段，以及这一步发出的命令。`offered` 是页面此刻算出可用的动作。 -/
def step (offered : Action → Bool) (stage : Stage) : Event → Stage × Option Send
  | .press action =>
    if pressable stage ∧ offered action then (.confirming action, none) else (stage, none)
  | .cancel =>
    match stage with
    | .confirming _ => (.idle, none)
    | other => (other, none)
  | .confirm idem =>
    match stage with
    | .confirming action => (.sending action idem, some ⟨action, idem⟩)
    | other => (other, none)
  | .lost idem =>
    match stage with
    | .sending action held => if held = idem then (.unsent action, none) else (stage, none)
    | other => (other, none)
  | .outcome idem result =>
    match stage with
    | .sending action held =>
      if held = idem then
        match result with
        | .running => (.sending action held, none)
        | .done => (.settled idem, none)
        | .refused => (.refused idem, none)
      else (stage, none)
    | other => (other, none)

/-- 走完一条事件序列：最后的阶段与依次发出的命令。 -/
def run (offered : Action → Bool) : Stage → List Event → Stage × List Send
  | stage, [] => (stage, [])
  | stage, event :: rest =>
    let (next, sent) := step offered stage event
    let (last, later) := run offered next rest
    (last, sent.toList ++ later)

/-- 一条事件序列里确认事件带的 idem。 -/
def confirmed : List Event → List Nat
  | [] => []
  | .confirm idem :: rest => idem :: confirmed rest
  | _ :: rest => confirmed rest

theorem cancel_never_sends (offered : Action → Bool) (stage : Stage) :
    (step offered stage .cancel).2 = none := by
  cases stage <;> rfl

theorem only_a_confirmation_sends (offered : Action → Bool) (stage : Stage) (event : Event)
    (sent : Send) (h : (step offered stage event).2 = some sent) :
    ∃ idem, event = .confirm idem ∧ stage = .confirming sent.action ∧ sent.idem = idem := by
  revert h
  cases event with
  | press action =>
    simp only [step]
    split <;> simp
  | cancel => simp [cancel_never_sends]
  | confirm idem =>
    cases stage <;> simp [step]
    intro h
    subst h
    exact ⟨rfl, rfl⟩
  | lost idem =>
    cases stage <;> simp [step]
    split <;> simp
  | outcome idem result =>
    cases stage <;> simp [step]
    split
    · cases result <;> simp
    · simp

theorem a_foreign_outcome_changes_nothing (offered : Action → Bool) (action : Action)
    (held idem : Nat) (result : Result) (h : held ≠ idem) :
    step offered (.sending action held) (.outcome idem result) = (.sending action held, none) := by
  simp [step, h]

theorem no_second_send_in_flight (offered : Action → Bool) (action : Action) (held : Nat)
    (event : Event) : (step offered (.sending action held) event).2 = none := by
  cases event with
  | press next => simp [step, pressable]
  | cancel => rfl
  | confirm idem => rfl
  | lost idem => simp only [step]; split <;> rfl
  | outcome idem result =>
    simp only [step]
    split
    · cases result <;> rfl
    · rfl

theorem an_unsent_action_can_be_pressed_again (offered : Action → Bool) (action : Action)
    (h : offered action = true) :
    step offered (.unsent action) (.press action) = (.confirming action, none) := by
  simp [step, pressable, h]

/-- 每一步至多发一条，且只有确认事件发：发出的 idem 依次是确认事件 idem 的子序列。 -/
theorem every_send_is_a_confirmation (offered : Action → Bool) (stage : Stage)
    (events : List Event) :
    ((run offered stage events).2.map Send.idem).Sublist (confirmed events) := by
  induction events generalizing stage with
  | nil => simp [run, confirmed]
  | cons event rest ih =>
    have tail := ih (step offered stage event).1
    have shape := only_a_confirmation_sends offered stage event
    simp only [run]
    cases hs : (step offered stage event).2 with
    | none =>
      rw [hs] at shape
      cases event <;> simp [confirmed] <;> first | exact tail | exact tail.cons _
    | some s =>
      obtain ⟨idem, rfl, -, hidem⟩ := shape s hs
      simp [confirmed, hidem]
      exact tail

/-- 回放用的可用动作：应用与恢复可用，核对不可用。 -/
def vectorOffered : Action → Bool
  | .apply | .restore => true
  | .reconcile => false

/-- 轨迹向量：每条是起始阶段、事件序列，以及 `run` 给出的结果。 -/
def vectors : List (Stage × List Event) :=
  [ (.idle, [.press .apply, .cancel]),
    (.idle, [.press .apply, .confirm 1, .outcome 2 .done, .outcome 1 .running, .outcome 1 .done]),
    (.idle, [.press .restore, .confirm 3, .lost 3, .press .restore, .confirm 4, .outcome 4 .refused]),
    (.idle, [.press .reconcile, .confirm 5]),
    (.settled 1, [.press .apply, .press .restore, .confirm 6, .press .apply, .confirm 7, .cancel]),
    (.idle, [.confirm 8, .outcome 8 .done, .lost 8]) ]

#eval vectors.map fun (start, events) => (start, events, run vectorOffered start events)

end Client.Views.Privacy
