-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# segmented：一个 Tab 站，选择跟随焦点

规定 `client/src/views/parts/segmented.ts` 的 `nextStop` 与 `tabStop`（APG Radio Group，roving tabindex），键表在 `client/spec/Views/Parts.lean` §7-4。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`segmented.test.ts` 判实现与这里的定义读出同样的格。

一格能不能选是一个布尔值：`why` 缺席即能选。选择跟随焦点，所以方向键落在哪一格就选中哪一格，不能选的格必须被跳过。

性质：

1. **方向键从不落在不能选的格上**，除非一格都选不了，那时留在原处（`an_arrow_never_lands_on_a_refused_cell`）。
2. **落点在控件之内**（`an_arrow_stays_inside`）。
3. **Tab 序列里恰好一站**：`tabStop` 只给一个格，空控件一站都没有（`an_empty_control_offers_no_stop`），给出的格在控件之内（`the_stop_is_a_cell`）；全部被拒时给第 0 格，原因还读得到（`a_refused_control_offers_its_first_cell`）。

咬得动的演示：不跳过不能选的格的写法（`stepBlind`）会落在一格被拒的格上（`a_blind_step_chooses_a_refused_cell`）。
-/

namespace Client.Views.Parts.Segmented

open Client.Views.Parts

/-- 第 `i` 格能不能选；控件之外的位置读作不能选。 -/
def choosable (cells : List Bool) (i : Nat) : Bool := cells.getD i false

/-- 从 `pos` 起按方向找下一个能选的格，至多走 `fuel` 步。 -/
def seek (cells : List Bool) (pos : Nat) (dir : Dir) : Nat → Option Nat
  | 0 => none
  | fuel + 1 =>
    if choosable cells (wrap cells.length pos dir) then some (wrap cells.length pos dir)
    else seek cells (wrap cells.length pos dir) dir fuel

/-- `nextStop`：绕一圈找能选的格，找不到就留在 `start`。 -/
def nextStop (cells : List Bool) (start : Nat) (dir : Dir) : Nat :=
  if cells.length = 0 then start
  else (seek cells (start % cells.length) dir cells.length).getD start

/-- 第一个能选的格。 -/
def firstFree : List Bool → Option Nat
  | [] => none
  | true :: _ => some 0
  | false :: rest => (firstFree rest).map (· + 1)

/-- `tabStop`：选中的格；没有选中的格时第一个能选的格；全部被拒时第 0 格；空控件没有。`chosen` 是 `held` 在格里的位置。 -/
def tabStop (cells : List Bool) (chosen : Option Nat) : Option Nat :=
  match chosen with
  | some i => if i < cells.length then some i else fallback cells
  | none => fallback cells
where
  fallback (cells : List Bool) : Option Nat :=
    match firstFree cells with
    | some i => some i
    | none => if cells.isEmpty then none else some 0

theorem seek_finds_a_free_cell (cells : List Bool) (dir : Dir) :
    ∀ fuel pos found, seek cells pos dir fuel = some found → choosable cells found = true := by
  intro fuel
  induction fuel with
  | zero => intro pos found h; simp [seek] at h
  | succ fuel ih =>
    intro pos found h
    simp only [seek] at h
    split at h
    · cases h; assumption
    · exact ih _ _ h

theorem seek_stays_inside (cells : List Bool) (dir : Dir) (h : 0 < cells.length) :
    ∀ fuel pos found, seek cells pos dir fuel = some found → found < cells.length := by
  intro fuel
  induction fuel with
  | zero => intro pos found hs; simp [seek] at hs
  | succ fuel ih =>
    intro pos found hs
    simp only [seek] at hs
    split at hs
    · cases hs; exact wrap_stays _ _ _ h
    · exact ih _ _ hs

theorem an_arrow_never_lands_on_a_refused_cell (cells : List Bool) (start : Nat) (dir : Dir) :
    nextStop cells start dir = start ∨ choosable cells (nextStop cells start dir) = true := by
  unfold nextStop
  split
  · exact Or.inl rfl
  · cases hs : seek cells (start % cells.length) dir cells.length with
    | none => exact Or.inl rfl
    | some found =>
      exact Or.inr (seek_finds_a_free_cell cells dir _ _ _ hs)

theorem an_arrow_stays_inside (cells : List Bool) (start : Nat) (dir : Dir)
    (h : start < cells.length) : nextStop cells start dir < cells.length := by
  unfold nextStop
  split
  · omega
  · cases hs : seek cells (start % cells.length) dir cells.length with
    | none => exact h
    | some found => exact seek_stays_inside cells dir (by omega) _ _ _ hs

theorem firstFree_inside : ∀ (cells : List Bool) (i : Nat), firstFree cells = some i → i < cells.length := by
  intro cells
  induction cells with
  | nil => intro i h; simp [firstFree] at h
  | cons head rest ih =>
    intro i h
    cases head with
    | true => simp [firstFree] at h; simp; omega
    | false =>
      simp only [firstFree, Option.map_eq_some_iff] at h
      obtain ⟨j, hj, rfl⟩ := h
      have := ih j hj
      simp; omega

theorem firstFree_none_of_refused : ∀ cells : List Bool, (∀ b ∈ cells, b = false) → firstFree cells = none := by
  intro cells
  induction cells with
  | nil => intro _; rfl
  | cons head rest ih =>
    intro all
    have hd : head = false := all head (List.mem_cons_self ..)
    subst hd
    simp only [firstFree, ih (fun b hb => all b (List.mem_cons_of_mem _ hb)), Option.map_none]

theorem an_empty_control_offers_no_stop (chosen : Option Nat) : tabStop [] chosen = none := by
  cases chosen <;> rfl

theorem the_stop_is_a_cell (cells : List Bool) (chosen : Option Nat) (stop : Nat)
    (h : tabStop cells chosen = some stop) : stop < cells.length := by
  have fallback_inside : tabStop.fallback cells = some stop → stop < cells.length := by
    intro hf
    unfold tabStop.fallback at hf
    cases hfree : firstFree cells with
    | some i => rw [hfree] at hf; exact firstFree_inside cells stop (by rw [hfree]; exact hf)
    | none =>
      rw [hfree] at hf
      cases cells with
      | nil => simp at hf
      | cons _ _ => simp at hf; subst hf; simp
  unfold tabStop at h
  cases chosen with
  | none => exact fallback_inside h
  | some i =>
    simp only at h
    split at h
    · cases h; assumption
    · exact fallback_inside h

theorem a_refused_control_offers_its_first_cell (cells : List Bool)
    (refused : ∀ b ∈ cells, b = false) (nonempty : cells ≠ []) : tabStop cells none = some 0 := by
  simp only [tabStop, tabStop.fallback, firstFree_none_of_refused cells refused]
  cases cells with
  | nil => exact absurd rfl nonempty
  | cons _ _ => rfl

/-- 被否决的写法：方向键走一格，不问那格能不能选。 -/
def stepBlind (cells : List Bool) (start : Nat) (dir : Dir) : Nat := wrap cells.length start dir

theorem a_blind_step_chooses_a_refused_cell :
    choosable [true, false, true] (stepBlind [true, false, true] 0 .forward) = false := by
  decide

example : nextStop [true, false, true] 0 .forward = 2 := by decide

example : nextStop [true, false, false] 0 .forward = 0 := by decide

example : tabStop [false, true, true] none = some 1 := by decide

end Client.Views.Parts.Segmented
