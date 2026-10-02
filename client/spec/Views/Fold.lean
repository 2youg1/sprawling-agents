-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# fold：一次只展开一项

规定两处折叠：设置树的枝（`client/src/views/settings/tree.svelte`，`client/Spec.lean` §7L、D53）与上手指南的步骤（`client/src/views/welcome.svelte`，§7G、D55）。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`tree.test.ts` 与 `guide.test.ts` 判实现与这里读出同一项。

状态是至多一项展开（`Option α`，`none` 是全部收起），所以「同时展开两项」在类型上写不出来。事件有两种：人按下一项的标题（`toggle`），以及当前所在的那一项移动了（`land`）——设置面开在一页之上、在树里选了另一组、指南因一步做完而前进到下一步。

性质：

1. **按下另一项，原来那一项收起**（`toggling_another_folds_the_open_one`）。
2. **再按展开着的那一项，全部收起**（`toggling_the_open_one_folds_it`）。
3. **任何事件序列以一次落点结束时，落到的那一项是展开的**（`the_last_landing_stays_open`）。
4. **任何事件序列之后至多一项展开**（`at_most_one_is_open`）。

咬得动的演示：每项各记一个布尔值的写法（`stepEach`）在按下第二项之后有两项展开（`a_flag_per_item_opens_two`）。
-/

namespace Client.Views.Fold

/-- 人与页面对折叠做的事。 -/
inductive Event (α : Type) where
  /-- 按下一项的标题。 -/
  | toggle (a : α)
  /-- 当前所在的那一项移到 `a`。 -/
  | land (a : α)

/-- 一个事件之后展开的那一项。 -/
def step {α : Type} [DecidableEq α] : Option α → Event α → Option α
  | some b, .toggle a => if a = b then none else some a
  | none, .toggle a => some a
  | _, .land a => some a

/-- 一串事件之后展开的那一项。 -/
def run {α : Type} [DecidableEq α] (start : Option α) (events : List (Event α)) : Option α :=
  events.foldl step start

theorem toggling_another_folds_the_open_one {α : Type} [DecidableEq α] (a b : α) (h : a ≠ b) :
    step (some b) (.toggle a) = some a := by
  simp [step, h]

theorem toggling_the_open_one_folds_it {α : Type} [DecidableEq α] (a : α) :
    step (some a) (.toggle a) = none := by
  simp [step]

theorem the_last_landing_stays_open {α : Type} [DecidableEq α] (s : Option α) (before : List (Event α)) (a : α) :
    run s (before ++ [.land a]) = some a := by
  simp [run, List.foldl_append, step]

theorem at_most_one_is_open {α : Type} [DecidableEq α] (s : Option α) (events : List (Event α)) :
    (run s events).toList.length ≤ 1 := by
  cases run s events <;> simp

/-- 每项各记一个布尔值、按下即取反的写法。 -/
def stepEach {α : Type} [DecidableEq α] (flags : α → Bool) (a : α) : α → Bool :=
  fun b => if b = a then !flags b else flags b

theorem a_flag_per_item_opens_two :
    let opened := stepEach (stepEach (fun _ : Bool => false) true) false
    opened true = true ∧ opened false = true :=
  ⟨rfl, rfl⟩

end Client.Views.Fold
