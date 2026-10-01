-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# strokes：一批输入只被收下一部分时，还按着什么

规定 `crates/desktop/src/platform/windows/strokes.rs`（`desktop::platform::windows::strokes`，`left_held` 与 `cut_short`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（D4）。

`SendInput` 只承诺按序插入、返回插入了几个，不承诺回滚。一批事件只被收下前 k 个时，前缀可能已经落在窗口上，修饰键可能停在按下。本 server 按收下的前缀算出仍按着的东西，按按下的相反次序补发它们的抬起；从不补发按下，从不重发整批。

一个事件是「某样东西（键、Unicode 单元、鼠标键）的按下或抬起」；指针移动与滚轮不按住任何东西，模型里不出现。抬起抹去最近一次还按着的同一样东西，于是一样东西按两次、抬一次仍按着一次。

四条性质：

1. **什么都没收下，就什么都不补**（`nothing_taken_nothing_lifted`）。
2. **补发的只是前缀里按下过的东西的抬起**（`every_lift_was_pressed`）：补发恒不让动作多做一步。
3. **按下又抬起的东西不再补发**（`a_finished_press_leaves_nothing_held`）。
4. **补发的个数不超过前缀里按下的个数**（`lifts_never_outnumber_presses`）。

**拿掉「抬起抹去按下」那一步，一次已经完成的点击也会被补一次抬起**（`withoutUp_lifts_a_finished_click`）：本模型咬得动的演示。
-/

namespace Desktop.Platform.Windows.Strokes

/-- 一个事件的边：按下或抬起。 -/
inductive Edge where
  | down
  | up
  deriving DecidableEq, Repr

/-- 一个事件：哪样东西、哪条边。 -/
structure Stroke (α : Type) where
  thing : α
  edge : Edge
  deriving DecidableEq, Repr

variable {α : Type} [DecidableEq α]

/-- 读一个事件之后还按着什么，按按下的次序；抬起抹去最近一次按下的同一样东西。 -/
def step (held : List α) (s : Stroke α) : List α :=
  match s.edge with
  | .down => held ++ [s.thing]
  | .up => (held.reverse.erase s.thing).reverse

/-- 前 `k` 个事件被收下之后还按着的东西。 -/
def held (strokes : List (Stroke α)) (k : Nat) : List α :=
  (strokes.take k).foldl step []

/-- `left_held`：补发的抬起，按按下的相反次序。 -/
def lifts (strokes : List (Stroke α)) (k : Nat) : List (Stroke α) :=
  (held strokes k).reverse.map fun thing => ⟨thing, .up⟩

/-- 前缀里按下过的东西。 -/
def pressed (strokes : List (Stroke α)) : Nat :=
  (strokes.filter fun s => s.edge = .down).length

theorem nothing_taken_nothing_lifted (strokes : List (Stroke α)) : lifts strokes 0 = [] := by
  simp [lifts, held]

theorem lifts_are_all_up (strokes : List (Stroke α)) (k : Nat) :
    ∀ s ∈ lifts strokes k, s.edge = .up := by
  intro s hs
  simp only [lifts, List.mem_map] at hs
  obtain ⟨_, _, rfl⟩ := hs
  rfl

theorem step_keeps (held : List α) (s : Stroke α) :
    ∀ x ∈ step held s, x ∈ held ∨ (x = s.thing ∧ s.edge = .down) := by
  intro x hx
  unfold step at hx
  split at hx
  · next h =>
    simp only [List.mem_append, List.mem_singleton] at hx
    rcases hx with old | new
    · exact Or.inl old
    · exact Or.inr ⟨new, h⟩
  · simp only [List.mem_reverse] at hx
    exact Or.inl (List.mem_reverse.mp (List.mem_of_mem_erase hx))

theorem fold_keeps (strokes : List (Stroke α)) :
    ∀ (acc : List α), ∀ x ∈ strokes.foldl step acc,
      x ∈ acc ∨ (⟨x, .down⟩ : Stroke α) ∈ strokes := by
  induction strokes with
  | nil => intro acc x hx; exact Or.inl hx
  | cons s rest ih =>
    intro acc x hx
    rcases ih (step acc s) x hx with inStep | later
    · rcases step_keeps acc s x inStep with old | ⟨rfl, down⟩
      · exact Or.inl old
      · refine Or.inr (List.mem_cons.mpr (Or.inl ?_))
        cases s
        simp_all
    · exact Or.inr (List.mem_cons_of_mem s later)

theorem every_lift_was_pressed (strokes : List (Stroke α)) (k : Nat) (x : α)
    (h : (⟨x, .up⟩ : Stroke α) ∈ lifts strokes k) :
    (⟨x, .down⟩ : Stroke α) ∈ strokes.take k := by
  simp only [lifts, List.mem_map, Stroke.mk.injEq] at h
  obtain ⟨y, hy, rfl, -⟩ := h
  rcases fold_keeps (strokes.take k) [] y (List.mem_reverse.mp hy) with none | found
  · simp at none
  · exact found

theorem a_finished_press_leaves_nothing_held (acc : List α) (x : α) :
    [(⟨x, .down⟩ : Stroke α), ⟨x, .up⟩].foldl step acc = acc := by
  simp [step, List.reverse_append]

theorem step_length_down (held : List α) (s : Stroke α) (h : s.edge = .down) :
    (step held s).length = held.length + 1 := by
  simp [step, h]

theorem step_length_up (held : List α) (s : Stroke α) (h : s.edge = .up) :
    (step held s).length ≤ held.length := by
  have shorter := List.length_erase_le (l := held.reverse) (a := s.thing)
  simp only [List.length_reverse] at shorter
  simpa [step, h] using shorter

theorem fold_length (strokes : List (Stroke α)) :
    ∀ (acc : List α), (strokes.foldl step acc).length ≤ acc.length + pressed strokes := by
  induction strokes with
  | nil => intro acc; simp [pressed]
  | cons s rest ih =>
    intro acc
    have more := ih (step acc s)
    rw [List.foldl_cons]
    cases h : s.edge with
    | down =>
      have one := step_length_down acc s h
      have hp : pressed (s :: rest) = pressed rest + 1 := by
        simp [pressed, h]
      rw [hp]
      omega
    | up =>
      have one := step_length_up acc s h
      have hp : pressed (s :: rest) = pressed rest := by
        simp [pressed, h]
      rw [hp]
      omega

theorem lifts_never_outnumber_presses (strokes : List (Stroke α)) (k : Nat) :
    (lifts strokes k).length ≤ pressed (strokes.take k) := by
  have := fold_length (strokes.take k) []
  simpa [lifts, held] using this

/-!
## 咬得动的演示

不让抬起抹去按下（只记按下），`a_finished_press_leaves_nothing_held` 不再成立：一次已经按下又抬起的左键，会被再补一次抬起，而那一刻人可能正按着它。
-/

def stepWithoutUp (held : List α) (s : Stroke α) : List α :=
  match s.edge with
  | .down => held ++ [s.thing]
  | .up => held

theorem withoutUp_lifts_a_finished_click :
    [(⟨0, .down⟩ : Stroke Nat), ⟨0, .up⟩].foldl stepWithoutUp [] = [0] := by
  decide

end Desktop.Platform.Windows.Strokes
