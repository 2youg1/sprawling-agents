-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# link：人关掉的城，链路停下

规定 `client/src/core/link.ts` 的 `advance` 在关闭这件事上的那一部分（`client/Spec.lean` §4-57c、D98）：`closing { mode }` 记下人用 `/quit` 要过的关闭，此后 socket 一断，链路进 `closed`，不走阶梯；只有人的 `retry` 让它重新打开，并忘掉这次关闭。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`link.test.ts` 的「a city the person closed」一组对每一种关闭方式、把每一种事件喂给 `closed` 的链路，判它与这里读出同样的答案。

模型只留判定关闭要用的东西：链路的七个阶段、链路记没记着关闭，与它收到的事件里的七类（`received` 拆成问候与其余的帧两类，`closed` 记作 `dropped`；读不出的帧与关闭无关，不在模型里）。

性质：

1. **`closed` 只认 `retry`**：从 `closed` 出发，任意一串不含 `retry` 的事件都让链路停在 `closed`（`closed_holds_until_retry`）。
2. **记着关闭时，socket 断开就停下**：链路不管在哪个连着或正在连的阶段，记着关闭时收到 `closed` 都进 `closed`，不进 `backoff`（`a_close_asked_for_stops_the_link`）。
3. **没要过关闭时，断开照旧走阶梯**（`an_outage_still_backs_off`）。
4. **`retry` 忘掉这次关闭**：从 `closed` 重试之后，链路在 `opening`，不再记着关闭（`retry_forgets_the_close`）。

咬得动的演示：不记关闭的写法（`advanceForgetful`）在人关掉城之后照旧进 `backoff`（`forgetting_the_close_backs_off`）。
-/

namespace Client.Core.Link

/-- 链路所处的阶段，与 `LinkState["kind"]` 一一对应。 -/
inductive Phase where
  | idle
  | opening
  | handshaking
  | live
  | backoff
  | refused
  | closed
  deriving DecidableEq, Repr

/-- 链路收到的事件；`welcome` 与 `frame` 是 `received` 的两类。 -/
inductive Event where
  | opened
  | welcome
  | frame
  | dropped
  | waitElapsed
  | retry
  | closing
  deriving DecidableEq, Repr

/-- 链路：阶段，与是否记着人要过的关闭。 -/
structure Link where
  phase : Phase
  closing : Bool
  deriving DecidableEq, Repr

/-- 停下的阶段：`refused` 与 `closed` 都只认 `retry`。 -/
def stopped (p : Phase) : Bool :=
  p == .refused || p == .closed

/-- `advance` 的关闭部分：停下的链路只认 `retry`，重试时忘掉关闭。 -/
def advance (l : Link) (e : Event) : Link :=
  if stopped l.phase then
    (match e with
      | .retry => ⟨.opening, false⟩
      | _ => l)
  else
    match e with
    | .opened => if l.phase == .opening then { l with phase := .handshaking } else l
    | .welcome => if l.phase == .handshaking then { l with phase := .live } else l
    | .frame => l
    | .dropped => if l.closing then { l with phase := .closed } else { l with phase := .backoff }
    | .waitElapsed => if l.phase == .backoff then { l with phase := .opening } else l
    | .retry => { l with phase := .opening }
    | .closing => { l with closing := true }

/-- 一串事件依次喂给链路。 -/
def run (l : Link) : List Event → Link
  | [] => l
  | e :: es => run (advance l e) es

theorem closed_step (c : Bool) (e : Event) (h : e ≠ .retry) :
    advance ⟨.closed, c⟩ e = ⟨.closed, c⟩ := by
  cases e <;> first | rfl | exact absurd rfl h

/-- 性质 1。 -/
theorem closed_holds_until_retry (c : Bool) (es : List Event) (h : ∀ e ∈ es, e ≠ .retry) :
    run ⟨.closed, c⟩ es = ⟨.closed, c⟩ := by
  induction es with
  | nil => rfl
  | cons e es ih =>
    have he : e ≠ .retry := h e (List.mem_cons_self ..)
    have hs : ∀ x ∈ es, x ≠ .retry := fun x hx => h x (List.mem_cons_of_mem e hx)
    simp only [run, closed_step c e he]
    exact ih hs

/-- 连着或正在连的阶段。 -/
def connecting (p : Phase) : Bool :=
  p == .opening || p == .handshaking || p == .live || p == .backoff

/-- 性质 2。 -/
theorem a_close_asked_for_stops_the_link (p : Phase) (h : connecting p = true) :
    advance ⟨p, true⟩ .dropped = ⟨.closed, true⟩ := by
  cases p <;> first | rfl | exact absurd h (by decide)

/-- 性质 3。 -/
theorem an_outage_still_backs_off (p : Phase) (h : connecting p = true) :
    advance ⟨p, false⟩ .dropped = ⟨.backoff, false⟩ := by
  cases p <;> first | rfl | exact absurd h (by decide)

/-- 性质 4。 -/
theorem retry_forgets_the_close (c : Bool) :
    advance ⟨.closed, c⟩ .retry = ⟨.opening, false⟩ := by
  cases c <;> rfl

/-- 不记关闭的写法：断开总是走阶梯。 -/
def advanceForgetful (l : Link) (e : Event) : Link :=
  match e with
  | .dropped => if stopped l.phase then l else { l with phase := .backoff }
  | _ => advance l e

/-- 咬得动的演示：人刚关掉城，不记关闭的写法照旧去敲那个关着的端口。 -/
theorem forgetting_the_close_backs_off :
    run ⟨.live, false⟩ [.closing] = ⟨.live, true⟩ ∧
    advanceForgetful ⟨.live, true⟩ .dropped = ⟨.backoff, true⟩ := by
  constructor <;> rfl

end Client.Core.Link
