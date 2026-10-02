-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 核心线程站在正常档之上，空转就降回

规定 `crates/sprawling/src/serving/standing.rs`（`bin::serving::standing`，形状：状态机）的安全阀 `Valve` 与一条核心线程 `CoreThread` 的档位（sprawling-SPEC.md 8-93）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

时刻是自然数：Rust 的 `saturating_duration_since` 在模型里就是自然数的截断减法。`Valve` 自己不读钟，时刻作参数传进来，所以它的判定在模型里与在 `serving::standing::tests` 里一样逐点可验。平台调用（升档、降档）的结果是参数。

四组性质：

* **降回是吸收态**——`verdict` 一旦是 `lower`，此后每一轮都还是 `lower`（D3：降回之后不再升）；
* **窗口没关上不判**——一轮结束时离窗口开出不足 `limit`，判定不变；
* **忙满一个窗口就判降回**，忙的时间不到窗口的 15/16 就不判；
* **线程只降一次、拒绝之后不再试**——设置为 `normal` 时从不升档；平台拒绝降档之后，此后的轮不再调用降档。
-/

namespace Sprawling.Serving.Standing

/-- 阀的判定（Rust：`Verdict`）。 -/
inductive Verdict where
  | keep
  | lower
  deriving Repr, DecidableEq

/-- 安全阀（Rust：`Valve`）：窗口从 `opened` 开出，`busy` 是窗口里忙的时长。 -/
structure Valve where
  limit : Nat
  opened : Nat
  busy : Nat
  verdict : Verdict
  deriving Repr, DecidableEq

/-- `Valve::new`：第一个窗口在 `now` 开出。 -/
def Valve.new (limit now : Nat) : Valve :=
  { limit, opened := now, busy := 0, verdict := .keep }

/-- `Valve::record`：线程在 `woke` 醒来、在 `slept` 再次阻塞。第一件结束在窗口开出 `limit` 之后的事关上窗口；关上时忙够 15/16 就判降回；下一个窗口从 `slept` 开出。 -/
def Valve.record (v : Valve) (woke slept : Nat) : Valve :=
  if slept - v.opened < v.limit then { v with busy := v.busy + (slept - woke) }
  else
    { v with
      verdict :=
        if (v.busy + (slept - woke)) * 16 ≥ (slept - v.opened) * 15 then .lower else v.verdict
      opened := slept
      busy := 0 }

theorem lower_is_absorbing (v : Valve) (woke slept : Nat) (h : v.verdict = .lower) :
    (v.record woke slept).verdict = .lower := by
  unfold Valve.record
  split <;> simp [h]

/-- 任意一串轮之后，降回仍是降回。 -/
theorem lower_stays_lower (v : Valve) (turns : List (Nat × Nat)) (h : v.verdict = .lower) :
    (turns.foldl (fun v t => v.record t.1 t.2) v).verdict = .lower := by
  induction turns generalizing v with
  | nil => simpa using h
  | cons t rest ih => exact ih _ (lower_is_absorbing v t.1 t.2 h)

theorem open_window_keeps_the_verdict (v : Valve) (woke slept : Nat)
    (h : slept - v.opened < v.limit) :
    (v.record woke slept).verdict = v.verdict := by
  simp [Valve.record, h]

/-- 一轮从窗口开出时醒来、到 `limit` 之后才阻塞：忙满了，判降回。 -/
theorem busy_through_the_window_is_lowered (limit now slept : Nat)
    (h : now + limit ≤ slept) :
    ((Valve.new limit now).record now slept).verdict = .lower := by
  have : ¬ slept - now < limit := by omega
  simp [Valve.record, Valve.new, this]
  omega

/-- 一个窗口里只忙一半，不判。 -/
example : (((Valve.new 10 0).record 0 5).record 15 20).verdict = .keep := by decide
example : (((Valve.new 10 0).record 0 9).record 9 10).verdict = .lower := by decide

/-- 人的设置（Rust：`accounting::person::CorePriority`）。 -/
inductive CorePriority where
  | raised
  | normal
  deriving Repr, DecidableEq

/-- 线程为什么留在正常档（Rust：`Held`）。拒绝的原因是平台的话，模型只记它是一次拒绝。 -/
inductive Held where
  | byTheSetting
  | refused
  | byTheValve
  deriving Repr, DecidableEq

/-- 这条线程实际站在哪一档（Rust：`Standing`）。 -/
inductive Standing where
  | raised
  | normal (held : Held)
  deriving Repr, DecidableEq

/-- `raise_this_thread`：`normal` 不调任何平台接口；`raised` 由平台答应或拒绝。 -/
def raiseThisThread (setting : CorePriority) (platformAccepts : Bool) : Standing :=
  match setting with
  | .normal => .normal .byTheSetting
  | .raised => if platformAccepts then .raised else .normal .refused

theorem normal_setting_never_raises (accepts : Bool) :
    raiseThisThread .normal accepts = .normal .byTheSetting := rfl

/-- 降档还欠着，或者平台拒绝过一次（Rust：`Lowering`）。 -/
inductive Lowering where
  | owed
  | refused
  deriving Repr, DecidableEq

/-- 一条核心线程（Rust：`CoreThread`）。 -/
structure CoreThread where
  standing : Standing
  valve : Valve
  lowering : Lowering
  deriving Repr, DecidableEq

/-- 这一轮之后是否调用降档：站在升档上、降档还欠着、阀判了降回。 -/
def CoreThread.asksToLower (t : CoreThread) (woke slept : Nat) : Bool :=
  t.standing == .raised && t.lowering == .owed && (t.valve.record woke slept).verdict == .lower

/-- `record_turn_lowering_when_busy`：`lowerAccepted` 是平台对这一次降档的答复。 -/
def CoreThread.record (t : CoreThread) (woke slept : Nat) (lowerAccepted : Bool) : CoreThread :=
  let valve := t.valve.record woke slept
  if t.asksToLower woke slept then
    if lowerAccepted then { t with valve, standing := .normal .byTheValve }
    else { t with valve, lowering := .refused }
  else { t with valve }

/-- 降回之后不再升，也不再调用降档：站在正常档上的线程不会再请求。 -/
theorem lowered_thread_asks_nothing (t : CoreThread) (held : Held) (woke slept : Nat)
    (h : t.standing = .normal held) : t.asksToLower woke slept = false := by
  simp [CoreThread.asksToLower, h]

/-- 平台拒绝过一次降档，此后每一轮都不再试。 -/
theorem refused_lowering_is_not_retried (t : CoreThread) (woke slept : Nat)
    (h : t.lowering = .refused) : t.asksToLower woke slept = false := by
  simp [CoreThread.asksToLower, h]

/-- 一轮从不把线程升上去：之后站在升档上，之前也站在升档上。 -/
theorem record_never_raises (t : CoreThread) (woke slept : Nat) (accepted : Bool)
    (h : (t.record woke slept accepted).standing = .raised) : t.standing = .raised := by
  unfold CoreThread.record at h
  split at h
  · split at h
    · simp at h
    · simpa using h
  · simpa using h

end Sprawling.Serving.Standing
