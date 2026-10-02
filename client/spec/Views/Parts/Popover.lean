-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# popover：多列 listbox

规定 `client/src/views/parts/popover.svelte`（多列 listbox，装在 `role="dialog"` 里），键表在 `client/spec/Views/Parts.lean` §7-5。本部件在这里覆盖平台的 Tab：Tab／Shift+Tab 换列（环绕）。

性质：

1. **换列把游标复位到第 0 行，列号留在列之内**（`a_new_column_starts_at_its_top`）。
2. **游标钳在当前列两端**（`a_move_stays_inside`）。
3. **Enter 应用的是当前列的游标行**（`enter_applies_the_cursor_of_this_column`）。
-/

namespace Client.Views.Parts.Popover

open Client.Views.Parts

/-- 弹层的状态：当前列与它的游标。 -/
structure State where
  column : Nat
  cursor : Nat
  deriving DecidableEq, Repr

/-- 弹层收的键。 -/
inductive Key where
  | down
  | up
  | home
  | finish
  | tab
  | shiftTab
  deriving DecidableEq, Repr

/-- 一个键之后的状态；`columns` 是列数，`rows c` 是第 `c` 列的行数。 -/
def press (columns : Nat) (rows : Nat → Nat) (s : State) : Key → State
  | .down => { s with cursor := clamp (rows s.column) s.cursor .forward }
  | .up => { s with cursor := clamp (rows s.column) s.cursor .backward }
  | .home => { s with cursor := 0 }
  | .finish => { s with cursor := rows s.column - 1 }
  | .tab => ⟨wrap columns s.column .forward, 0⟩
  | .shiftTab => ⟨wrap columns s.column .backward, 0⟩

/-- Enter 应用哪一行：当前列与它的游标。 -/
def applied (s : State) : Nat × Nat := (s.column, s.cursor)

theorem a_new_column_starts_at_its_top (columns : Nat) (rows : Nat → Nat) (s : State)
    (h : 0 < columns) (key : Key) (turn : key = .tab ∨ key = .shiftTab) :
    (press columns rows s key).cursor = 0 ∧ (press columns rows s key).column < columns := by
  rcases turn with rfl | rfl
  · exact ⟨rfl, wrap_stays columns s.column .forward h⟩
  · exact ⟨rfl, wrap_stays columns s.column .backward h⟩

theorem a_move_stays_inside (columns : Nat) (rows : Nat → Nat) (s : State)
    (inside : s.cursor < rows s.column) (key : Key) (vertical : key ≠ .tab ∧ key ≠ .shiftTab) :
    (press columns rows s key).cursor < rows (press columns rows s key).column := by
  cases key with
  | down => exact clamp_stays (rows s.column) s.cursor .forward inside
  | up => exact clamp_stays (rows s.column) s.cursor .backward inside
  | home => simp only [press]; omega
  | finish => simp only [press]; omega
  | tab => exact absurd rfl vertical.1
  | shiftTab => exact absurd rfl vertical.2

theorem enter_applies_the_cursor_of_this_column (s : State) : applied s = (s.column, s.cursor) := rfl

end Client.Views.Parts.Popover
