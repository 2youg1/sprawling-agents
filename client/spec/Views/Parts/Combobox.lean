-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# combobox：一层 listbox 弹层

规定 `client/src/views/parts/combobox.svelte`（APG Combobox，listbox 弹层），键表在 `client/spec/Views/Parts.lean` §7-5。模型是规格，今天的实现欠而未还的几处（Home／End、Tab 关闭弹层、过滤框的角色）点名在 `client/Spec.lean` §4 的 §7-8。

状态是：弹层开不开、游标在第几行、过滤词、焦点在哪，以及**生效的值**（`chosen`，`aria-selected` 读它）。游标由持焦元素的 `aria-activedescendant` 承担，从不写进 `aria-selected`。

性质：

1. **游标走动不改生效的值**（`the_cursor_is_not_the_selection`）：`aria-selected` 在 combobox 与 popover 里说同一件事，「这是当前生效的值」。
2. **游标钳在列表两端**（`a_move_stays_inside`）。
3. **打一个字过滤，游标复位到第 0 行**（`typing_resets_the_cursor`）。
4. **Enter 采纳游标行、关上弹层、焦点回触发器**（`enter_adopts_the_cursor`）。
5. **Escape 关上弹层、清空过滤词、焦点回触发器、不改生效的值**（`escape_closes_and_keeps_the_value`）。
6. **Tab 关上弹层并让焦点正常离开**（`tab_lets_the_focus_leave`）。
-/

namespace Client.Views.Parts.Combobox

open Client.Views.Parts

/-- 焦点在哪。 -/
inductive Place where
  | trigger
  | field
  | away
  deriving DecidableEq, Repr

/-- 组合框的状态。 -/
structure State where
  opened : Bool
  cursor : Nat
  filter : List Char
  focus : Place
  chosen : Option Nat
  deriving DecidableEq, Repr

/-- 弹层开着时收的键。 -/
inductive Key where
  | down
  | up
  | home
  | finish
  | enter
  | escape
  | tab
  | printable (c : Char)
  deriving DecidableEq, Repr

/-- 一个键之后的状态；`rows` 是一个过滤词留下几行。 -/
def press (rows : List Char → Nat) (s : State) : Key → State
  | .down => { s with cursor := clamp (rows s.filter) s.cursor .forward }
  | .up => { s with cursor := clamp (rows s.filter) s.cursor .backward }
  | .home => { s with cursor := 0 }
  | .finish => { s with cursor := rows s.filter - 1 }
  | .enter => { s with opened := false, focus := .trigger, chosen := some s.cursor }
  | .escape => { s with opened := false, filter := [], focus := .trigger }
  | .tab => { s with opened := false, focus := .away }
  | .printable c => { s with filter := s.filter ++ [c], cursor := 0 }

/-- 只移动游标的键。 -/
def moves : Key → Bool
  | .down => true
  | .up => true
  | .home => true
  | .finish => true
  | .enter => false
  | .escape => false
  | .tab => false
  | .printable _ => false

theorem the_cursor_is_not_the_selection (rows : List Char → Nat) (s : State) (key : Key)
    (h : moves key = true) : (press rows s key).chosen = s.chosen := by
  cases key <;> simp_all [moves, press]

theorem a_move_stays_inside (rows : List Char → Nat) (s : State) (key : Key)
    (h : moves key = true) (inside : s.cursor < rows s.filter) :
    (press rows s key).cursor < rows (press rows s key).filter := by
  cases key with
  | down => exact clamp_stays (rows s.filter) s.cursor .forward inside
  | up => exact clamp_stays (rows s.filter) s.cursor .backward inside
  | home => simp only [press]; omega
  | finish => simp only [press]; omega
  | enter => simp [moves] at h
  | escape => simp [moves] at h
  | tab => simp [moves] at h
  | printable _ => simp [moves] at h

theorem typing_resets_the_cursor (rows : List Char → Nat) (s : State) (c : Char) :
    (press rows s (.printable c)).cursor = 0 := rfl

theorem enter_adopts_the_cursor (rows : List Char → Nat) (s : State) :
    press rows s .enter = { s with opened := false, focus := .trigger, chosen := some s.cursor } := rfl

theorem escape_closes_and_keeps_the_value (rows : List Char → Nat) (s : State) :
    (press rows s .escape).opened = false ∧ (press rows s .escape).filter = [] ∧
      (press rows s .escape).focus = .trigger ∧ (press rows s .escape).chosen = s.chosen :=
  ⟨rfl, rfl, rfl, rfl⟩

theorem tab_lets_the_focus_leave (rows : List Char → Nat) (s : State) :
    (press rows s .tab).opened = false ∧ (press rows s .tab).focus = .away := ⟨rfl, rfl⟩

end Client.Views.Parts.Combobox
