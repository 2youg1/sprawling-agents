-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::concurrency

规定 `concurrency`（`crates/gateway/src/concurrency.rs`）：一个端点的并发名额，纯判定，只读注入的时刻与调用的结果。本文件是 `crates/gateway/Spec.lean` 的一个分部，形状与常量的理由在 D17。

模型是一个端点的名额状态在一条事件轨迹上的演变。事件五种：取（take）、还（give back）、被限流（429，带或不带 `Retry-After`）、一次成功、时间流逝（tick）。每个带时刻的事件自带它发生的时刻，状态不持有时钟，所以三个平台相同。

下面证明五条性质，各在一条轨迹上量化：
1. 取到的名额从不让在用数超过当下的上限；没有收窄的轨迹上，在用数始终不超过上限。收窄不收回已在飞的调用，所以「在用数 ≤ 上限」在一次收窄之后可以暂时不成立，直到在飞的调用陆续归还——这是对 D17「名额在用不超过上限」的精确读法。
2. 上限始终在 1 到配置值之间。
3. 在 `Retry-After` 给出的时刻之前，上限不会变大；上限变大的唯一一步是那个时刻之后的一次成功。
4. 连续 `WIDEN_AFTER` 次成功（在那个时刻之后）让上限加一，直到配置值。
5. 一次取答「取到」「已满」或「等到某一刻」：已满只在名额都在用时答（由一次归还唤醒），等到的时刻严格晚于此刻，所以调用方不会忙等。
-/

namespace Gateway.Concurrency

/-- 连续多少次成功之后上限加一（`concurrency::WIDEN_AFTER`）。 -/
def WIDEN_AFTER : Nat := 8

/-- 一个端点的名额状态（`concurrency::Permits`）。`cap` 是配置的上限，不随事件变。 -/
structure Permits where
  cap : Nat
  limit : Nat
  inUse : Nat
  streak : Nat
  holdUntil : Nat
  deriving Repr

/-- 一次取的回答（`concurrency::Take`）。 -/
inductive Answer where
  | granted
  | full
  | waitUntil (t : Nat)
  deriving DecidableEq, Repr

/-- 一条轨迹上的一步。 -/
inductive Event where
  | take (now : Nat)
  | giveBack
  | rateLimited (now : Nat) (retryAfter : Option Nat)
  | success (now : Nat)
  | tick (now : Nat)

/-- 状态良好：上限在 1 到配置值之间，连胜数短于放宽所需。 -/
def Valid (s : Permits) : Prop :=
  1 ≤ s.limit ∧ s.limit ≤ s.cap ∧ s.streak < WIDEN_AFTER

def answer (s : Permits) (now : Nat) : Answer :=
  if now < s.holdUntil then .waitUntil s.holdUntil
  else if s.inUse < s.limit then .granted
  else .full

def narrow (s : Permits) (now : Nat) (retryAfter : Option Nat) : Permits :=
  { s with
    limit := max 1 (s.limit / 2)
    streak := 0
    holdUntil := match retryAfter with
      | some d => max s.holdUntil (now + d)
      | none => s.holdUntil }

/-- 一次成功：连胜加一；凑满 `WIDEN_AFTER` 且已过 `holdUntil` 且未到配置值时上限加一、连胜归零；
凑满却还不能放宽时连胜停在差一次的位置，于是那个时刻之后的第一次成功就放宽。 -/
def succeed (s : Permits) (now : Nat) : Permits :=
  if s.streak + 1 < WIDEN_AFTER then { s with streak := s.streak + 1 }
  else if s.holdUntil ≤ now ∧ s.limit < s.cap then { s with limit := s.limit + 1, streak := 0 }
  else s

def step (s : Permits) : Event → Permits
  | .take now =>
    match answer s now with
    | .granted => { s with inUse := s.inUse + 1 }
    | .full => s
    | .waitUntil _ => s
  | .giveBack => { s with inUse := s.inUse - 1 }
  | .rateLimited now ra => narrow s now ra
  | .success now => succeed s now
  | .tick _ => s

def run (s : Permits) (trace : List Event) : Permits := trace.foldl step s

theorem run_cons (s : Permits) (e : Event) (es : List Event) :
    run s (e :: es) = run (step s e) es := rfl

/-! ### 性质 5：取不会让调用方忙等 -/

theorem wait_is_later (s : Permits) (now t : Nat) (h : answer s now = .waitUntil t) : now < t := by
  unfold answer at h
  split at h
  · cases h; assumption
  · split at h <;> cases h

theorem full_only_when_all_in_use (s : Permits) (now : Nat) (h : answer s now = .full) :
    s.limit ≤ s.inUse := by
  unfold answer at h
  split at h
  · cases h
  · split at h
    · cases h
    · omega

/-! ### 性质 1：取到的名额不越过上限 -/

theorem granted_within_limit (s : Permits) (now : Nat) (h : answer s now = .granted) :
    (step s (.take now)).inUse ≤ s.limit := by
  simp only [step, h]
  unfold answer at h
  split at h
  · cases h
  · split at h
    · omega
    · cases h

def Narrows : Event → Bool
  | .rateLimited _ _ => true
  | _ => false

theorem step_keeps_within (s : Permits) (e : Event) (hn : Narrows e = false)
    (hw : s.inUse ≤ s.limit) :
    (step s e).inUse ≤ (step s e).limit := by
  cases e with
  | take now =>
    have hg := granted_within_limit s now
    cases ha : answer s now with
    | granted =>
      have h := hg ha
      simp only [step, ha] at h ⊢
      exact h
    | full => simp only [step, ha]; exact hw
    | waitUntil t => simp only [step, ha]; exact hw
  | giveBack => simp only [step]; omega
  | rateLimited now ra => simp [Narrows] at hn
  | success now =>
    simp only [step, succeed]
    split
    · exact hw
    · split
      · simp only; omega
      · exact hw
  | tick now => exact hw

/-! ### 性质 2：上限在 1 到配置值之间 -/

theorem step_valid (s : Permits) (e : Event) (hv : Valid s) : Valid (step s e) := by
  obtain ⟨h1, h2, h3⟩ := hv
  cases e with
  | take now =>
    simp only [step]
    split <;> exact ⟨h1, h2, h3⟩
  | giveBack => exact ⟨h1, h2, h3⟩
  | rateLimited now ra =>
    refine ⟨?_, ?_, ?_⟩ <;> simp only [step, narrow, WIDEN_AFTER] <;> omega
  | success now =>
    simp only [step, succeed]
    split
    · exact ⟨h1, h2, by assumption⟩
    · split
      · rename_i h
        refine ⟨?_, ?_, ?_⟩ <;> simp only [WIDEN_AFTER] <;> omega
      · exact ⟨h1, h2, h3⟩
  | tick now => exact ⟨h1, h2, h3⟩

theorem run_valid (s : Permits) (trace : List Event) (hv : Valid s) : Valid (run s trace) := by
  induction trace generalizing s with
  | nil => exact hv
  | cons e es ih => rw [run_cons]; exact ih _ (step_valid s e hv)

theorem run_keeps_within (s : Permits) (trace : List Event) (hv : Valid s)
    (hw : s.inUse ≤ s.limit) (hn : ∀ e ∈ trace, Narrows e = false) :
    (run s trace).inUse ≤ (run s trace).limit := by
  induction trace generalizing s with
  | nil => exact hw
  | cons e es ih =>
    rw [run_cons]
    exact ih _ (step_valid s e hv)
      (step_keeps_within s e (hn e (List.Mem.head _)) hw)
      (fun e' he' => hn e' (List.Mem.tail _ he'))

/-! ### 性质 3：`Retry-After` 的时刻之前不放宽 -/

theorem holdUntil_never_earlier (s : Permits) (e : Event) :
    s.holdUntil ≤ (step s e).holdUntil := by
  cases e with
  | take now => simp only [step]; split <;> simp
  | giveBack => simp [step]
  | rateLimited now ra =>
    cases ra <;> simp only [step, narrow] <;> omega
  | success now =>
    simp only [step, succeed]
    split
    · simp
    · split <;> simp
  | tick now => simp [step]

theorem widens_only_after_hold (s : Permits) (e : Event) (hv : Valid s)
    (h : s.limit < (step s e).limit) :
    ∃ now, e = .success now ∧ s.holdUntil ≤ now := by
  obtain ⟨h1, _, _⟩ := hv
  cases e with
  | take now =>
    simp only [step] at h
    split at h <;> simp at h
  | giveBack => simp [step] at h
  | rateLimited now ra => simp only [step, narrow] at h; omega
  | success now =>
    refine ⟨now, rfl, ?_⟩
    simp only [step, succeed] at h
    split at h
    · simp at h
    · split at h
      · rename_i hc; exact hc.1
      · simp at h
  | tick now => simp [step] at h

/-- 一条轨迹上每一步都发生在 `holdUntil` 之前时，上限不变大。 -/
def Before (hold : Nat) : Event → Prop
  | .take now => now < hold
  | .giveBack => True
  | .rateLimited now _ => now < hold
  | .success now => now < hold
  | .tick now => now < hold

theorem no_widening_before_hold (s : Permits) (trace : List Event) (hv : Valid s)
    (hb : ∀ e ∈ trace, Before s.holdUntil e) :
    (run s trace).limit ≤ s.limit := by
  induction trace generalizing s with
  | nil => exact Nat.le_refl _
  | cons e es ih =>
    rw [run_cons]
    have hs : (step s e).limit ≤ s.limit := by
      apply Nat.le_of_not_lt
      intro hlt
      obtain ⟨now, he, hle⟩ := widens_only_after_hold s e hv hlt
      have := hb e (List.Mem.head _)
      rw [he] at this
      simp only [Before] at this
      omega
    have hrest := ih (step s e) (step_valid s e hv) (fun e' he' =>
      match e', hb e' (List.Mem.tail _ he') with
      | .take _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .giveBack, h => h
      | .rateLimited _ _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .success _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e)
      | .tick _, h => Nat.lt_of_lt_of_le h (holdUntil_never_earlier s e))
    omega

/-! ### 性质 4：连续 `WIDEN_AFTER` 次成功放宽一格 -/

theorem successes_widen_by_one (s : Permits) (now : Nat) (hz : s.streak = 0)
    (hh : s.holdUntil ≤ now) (hc : s.limit < s.cap) :
    (run s (List.replicate WIDEN_AFTER (.success now))).limit = s.limit + 1 := by
  obtain ⟨cap, limit, inUse, streak, hold⟩ := s
  simp only at hz hh hc
  subst hz
  simp [WIDEN_AFTER, List.replicate, run, step, succeed, hh, hc]

theorem step_keeps_cap (s : Permits) (e : Event) : (step s e).cap = s.cap := by
  cases e with
  | take now => simp only [step]; split <;> rfl
  | giveBack => rfl
  | rateLimited now ra => rfl
  | success now =>
    simp only [step, succeed]
    split
    · rfl
    · split <;> rfl
  | tick now => rfl

theorem run_keeps_cap (s : Permits) (trace : List Event) : (run s trace).cap = s.cap := by
  induction trace generalizing s with
  | nil => rfl
  | cons e es ih => rw [run_cons, ih, step_keeps_cap]

/-- 成功再多，上限也停在配置值。 -/
theorem successes_stop_at_cap (s : Permits) (trace : List Event) (hv : Valid s) :
    (run s trace).limit ≤ s.cap := by
  rw [← run_keeps_cap s trace]
  exact (run_valid s trace hv).2.1

end Gateway.Concurrency
