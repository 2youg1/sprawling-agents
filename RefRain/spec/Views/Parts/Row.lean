-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# row：一串各自可达的行，加上方向键

规定 `client/src/views/parts/row.svelte` 的 `RowList`，键表在 `client/spec/Views/Parts.lean` §7-4。它没有 APG 部件模式：每一行仍是一个 Tab 站，方向键是加法不是替换。信箱面里的条目（`client/spec/Views/Workspace.lean` §7-11 的 j/k）用同一种走法。

性质：

1. **两端不环绕**（`the_last_row_holds`、`the_first_row_holds`），且落点在列表之内（`a_key_stays_inside`）。
2. **落在文本框、`<select>` 或可编辑区域上的键不接管**（`a_field_keeps_its_keys`）：那些键在那个控件里已经有意思了。
3. **Tab 归平台**（`tab_is_the_platforms`）：逐行走照旧。

咬得动的演示：用环绕代替钳住的写法从末行一步回到首行（`wrapping_jumps_to_the_top`）。
-/

namespace Client.Views.Parts.Row

open Client.Views.Parts

/-- 键落在哪一类元素上。 -/
inductive Target where
  | row
  | text
  | select
  | editable
  deriving DecidableEq, Repr

/-- `RowList` 读的键。 -/
inductive Key where
  | down
  | up
  | home
  | finish
  | tab
  deriving DecidableEq, Repr

/-- 这个键由 `RowList` 接管吗。 -/
def taken : Target → Key → Bool
  | .row, .down => true
  | .row, .up => true
  | .row, .home => true
  | .row, .finish => true
  | .row, .tab => false
  | .text, _ => false
  | .select, _ => false
  | .editable, _ => false

/-- 接管的键之后焦点在第几行。 -/
def move (total pos : Nat) : Key → Nat
  | .down => clamp total pos .forward
  | .up => clamp total pos .backward
  | .home => 0
  | .finish => total - 1
  | .tab => pos

theorem a_key_stays_inside (total pos : Nat) (key : Key) (h : pos < total) :
    move total pos key < total := by
  cases key with
  | down => exact clamp_stays total pos .forward h
  | up => exact clamp_stays total pos .backward h
  | home => simp only [move]; omega
  | finish => simp only [move]; omega
  | tab => exact h

theorem the_last_row_holds (total : Nat) : move total (total - 1) .down = total - 1 :=
  clamp_holds_the_last total

theorem the_first_row_holds (total : Nat) : move total 0 .up = 0 := rfl

theorem a_field_keeps_its_keys (key : Key) :
    taken .text key = false ∧ taken .select key = false ∧ taken .editable key = false := by
  cases key <;> decide

theorem tab_is_the_platforms (target : Target) : taken target .tab = false := by
  cases target <;> rfl

theorem wrapping_jumps_to_the_top : wrap 1000 999 .forward = 0 := by decide

end Client.Views.Parts.Row
