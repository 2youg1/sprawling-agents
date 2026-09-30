-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# devloop：改一处、看一眼、再决定，而且必有终点

规定 `crates/browser/src/devloop.rs`（`browser::devloop`，`DevLoop::observe`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个回路记着看了几眼、连续几眼没变、上一眼是什么。每一眼得到四个结局之一：页面报错（`complained`）、连续 `QUIET_LOOKS` 眼没变（`settled`）、看满 `LOOKS_MAX` 眼仍在变（`gaveUp`）、再看一眼（`lookAgain`）。两个常量把「不收敛」变成一个结局，而不是一段时间。结局之后再观察是调用方的错，报 `E_LOOP_SUSPECTED`，因为调用方持着 `Step`。

`browser` 工具的 `snapshot` 动作产出的文本就是一眼的 `text`，`console` 里出现过 error 级别的条目就是 `complained`；「改一处、看一眼、再决定」在工具层闭合，`Step` 作为工具结果回给模型。这是接线而不是新判定。

三条性质：

1. **再看一眼只在预算之内**（`lookAgain_within_budget`）：答 `lookAgain` 时已看的眼数小于 `LOOKS_MAX`。
2. **预算用完就不再答**（`spent_refused`）：看满 `LOOKS_MAX` 眼之后的观察一律被拒。每一次被接受的观察把眼数加一（`observe_counts`），所以从新回路起，任何观察序列至多 `LOOKS_MAX` 眼就到达一个结局或被拒。
3. **拿掉「看满即放弃」，第八眼仍会答再看一眼**（`withoutGiveUp_looks_again_at_the_limit`）：本模型咬得动的演示。
-/

namespace Browser.Devloop

def LOOKS_MAX : Nat := 8
def QUIET_LOOKS : Nat := 2

structure Observation where
  text : String
  complained : Bool

inductive Step where
  | settled (looks : Nat)
  | lookAgain (looks : Nat)
  | complained (looks : Nat)
  | gaveUp (looks : Nat)
  deriving DecidableEq, Repr

structure Loop where
  looks : Nat
  same : Nat
  last : Option String

def Loop.new : Loop := ⟨0, 0, none⟩

/-- 连续几眼没变：这一眼与上一眼相同就加一，否则归零。 -/
def Loop.sameRun (l : Loop) (o : Observation) : Nat :=
  if l.last = some o.text then l.same + 1 else 0

/-- 一眼没报错时的结局：先问静没静下来，再问看满没有。 -/
def verdict (looks same : Nat) : Step :=
  if same ≥ QUIET_LOOKS then .settled looks
  else if looks ≥ LOOKS_MAX then .gaveUp looks
  else .lookAgain looks

/-- `observe`：`none` 就是 `E_LOOP_SUSPECTED`。 -/
def Loop.observe (l : Loop) (o : Observation) : Option (Step × Loop) :=
  if l.looks ≥ LOOKS_MAX then none
  else if o.complained then some (.complained (l.looks + 1), { l with looks := l.looks + 1 })
  else some (verdict (l.looks + 1) (l.sameRun o), ⟨l.looks + 1, l.sameRun o, some o.text⟩)

theorem spent_refused (l : Loop) (o : Observation) (h : l.looks ≥ LOOKS_MAX) :
    l.observe o = none := by
  simp [Loop.observe, h]

theorem observe_counts {l l' : Loop} {o : Observation} {s : Step}
    (h : l.observe o = some (s, l')) : l'.looks = l.looks + 1 := by
  unfold Loop.observe at h
  by_cases spent : l.looks ≥ LOOKS_MAX
  · simp [spent] at h
  · by_cases c : o.complained = true
    · simp only [spent, c, if_false, if_true, Option.some.injEq, Prod.mk.injEq] at h
      rw [← h.2]
    · simp only [spent, c, Bool.false_eq_true, if_false, Option.some.injEq, Prod.mk.injEq] at h
      rw [← h.2]

theorem verdict_lookAgain {looks same n : Nat} (h : verdict looks same = .lookAgain n) :
    n = looks ∧ looks < LOOKS_MAX := by
  unfold verdict at h
  by_cases quiet : same ≥ QUIET_LOOKS
  · simp [quiet] at h
  · by_cases full : looks ≥ LOOKS_MAX
    · simp [quiet, full] at h
    · simp only [quiet, full, if_false, Step.lookAgain.injEq] at h
      omega

theorem lookAgain_within_budget {l l' : Loop} {o : Observation} {n : Nat}
    (h : l.observe o = some (.lookAgain n, l')) : n < LOOKS_MAX := by
  unfold Loop.observe at h
  by_cases spent : l.looks ≥ LOOKS_MAX
  · simp [spent] at h
  · by_cases c : o.complained = true
    · simp [spent, c] at h
    · simp only [spent, c, Bool.false_eq_true, if_false, Option.some.injEq, Prod.mk.injEq] at h
      have := verdict_lookAgain h.1
      omega

/-!
## 咬得动的演示

拿掉「看满 `LOOKS_MAX` 眼即放弃」那一支，`lookAgain_within_budget` 不再成立：第八眼仍答再看一眼，回路的终点只剩下调用方下一次被拒。
-/

def Loop.observeWithoutGiveUp (l : Loop) (o : Observation) : Option (Step × Loop) :=
  if l.looks ≥ LOOKS_MAX then none
  else if o.complained then some (.complained (l.looks + 1), { l with looks := l.looks + 1 })
  else if l.sameRun o ≥ QUIET_LOOKS then
    some (.settled (l.looks + 1), ⟨l.looks + 1, l.sameRun o, some o.text⟩)
  else some (.lookAgain (l.looks + 1), ⟨l.looks + 1, l.sameRun o, some o.text⟩)

theorem withoutGiveUp_looks_again_at_the_limit :
    ((Loop.observeWithoutGiveUp ⟨7, 0, some "a"⟩ ⟨"b", false⟩).map Prod.fst) =
      some (.lookAgain 8) := by
  rfl

end Browser.Devloop
