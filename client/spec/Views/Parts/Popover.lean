-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# popover：多列 listbox

规定 `client/src/views/parts/popover.svelte`（多列 listbox，装在 `role="dialog"` 里），键表在 `client/spec/Views/Parts.lean` §7-5。本部件在这里覆盖平台的 Tab：Tab／Shift+Tab 换列（环绕）。

**两个方向共用一张状态（列号＋游标），键表按方向取一份**：`layout="rows"` 把各列堆成一张表的各行（行首是列名），于是 ←／→ 在行内走、↓／↑ 换行（与 Tab 同一动作）；默认方向下 ↓／↑ 在列内走、←／→ 不被本部件接管，留给调用方的光标。

性质：

1. **换列把游标复位到第 0 行，列号留在列之内**（`a_new_column_starts_at_its_top`）。
2. **游标钳在当前列两端**（`a_move_stays_inside`）。
3. **Enter 应用的是当前列的游标行**（`enter_applies_the_cursor_of_this_column`）。
4. **能装进视口的活动行完整可见**（`a_fitting_row_is_visible`）。
5. **非空视口与非空活动行相交**（`a_nonempty_viewport_reaches_the_row`）。
6. **表的行内走法留在本行**（`a_side_step_stays_in_its_row`）。
7. **换行把游标复位到第 0 行，行号留在表内**（`a_step_between_rows_starts_at_its_top`）。
8. **首选一侧放得下，弹层就开在那一侧**（`a_fitting_side_is_kept`）。
9. **换到另一侧只为更多的空间**（`a_flip_gains_room`）。

列表持焦与 `bind` 文本框持焦共用活动行的可见性规则，滚动不取焦、不移动其他列或外层页面。坐标模型使用有序的非负离散单位；浏览器检查负责 DOM 的实际行高、亚像素坐标、焦点保持与按键到滚动的接线。

`reveal` 与 `opensOn` 这两条几何规则也管 `combobox.svelte` 的单列表：它的游标行按同一个 `reveal` 滚进视野，它的弹层按同一个 `opensOn` 选上下。两个部件读的是同一份实现 `client/src/views/parts/layer.ts`，派生检查是旁边的 `layer.test.ts`。
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
  | left
  | right
  | home
  | finish
  | tab
  | shiftTab
  deriving DecidableEq, Repr

/-- 一个键之后的状态；`columns` 是列数，`rows c` 是第 `c` 列的行数。
左右键不属这个方向：调用方（文本框的挂号）自己留着它们。 -/
def press (columns : Nat) (rows : Nat → Nat) (s : State) : Key → State
  | .down => { s with cursor := clamp (rows s.column) s.cursor .forward }
  | .up => { s with cursor := clamp (rows s.column) s.cursor .backward }
  | .left => s
  | .right => s
  | .home => { s with cursor := 0 }
  | .finish => { s with cursor := rows s.column - 1 }
  | .tab => ⟨wrap columns s.column .forward, 0⟩
  | .shiftTab => ⟨wrap columns s.column .backward, 0⟩

/-- 堆成表的那个方向的键表：←／→ 在行内走，↓／↑ 换行并复位到第 0 行。 -/
def pressRow (lists : Nat) (rows : Nat → Nat) (s : State) : Key → State
  | .left => { s with cursor := clamp (rows s.column) s.cursor .backward }
  | .right => { s with cursor := clamp (rows s.column) s.cursor .forward }
  | .down => ⟨wrap lists s.column .forward, 0⟩
  | .up => ⟨wrap lists s.column .backward, 0⟩
  | .home => { s with cursor := 0 }
  | .finish => { s with cursor := rows s.column - 1 }
  | .tab => ⟨wrap lists s.column .forward, 0⟩
  | .shiftTab => ⟨wrap lists s.column .backward, 0⟩

/-- D1：DOM 更新后让活动行可见，只滚动当前列；最近的可见边界保留其余行的位置，过高的行露出行首。
`top` 与 `bottom` 是行在列内容中的边界，`scroll` 是视口起点，`height` 是视口高度。 -/
def reveal (top bottom scroll height : Nat) : Nat :=
  if top < scroll ∨ height < bottom - top then top
  else if scroll + height < bottom then bottom - height
  else scroll

/-- 对每一个能容纳整行的视口，滚动后的两条边界包住活动行。 -/
theorem a_fitting_row_is_visible (top bottom scroll height : Nat)
    (fits : bottom ≤ top + height) :
    reveal top bottom scroll height ≤ top ∧
      bottom ≤ reveal top bottom scroll height + height := by
  unfold reveal
  split
  · omega
  · split <;> omega

/-- 任意初始滚动位置下，非空视口都与非空行相交，包括视口容纳不了整行的情况。 -/
theorem a_nonempty_viewport_reaches_the_row (top bottom scroll height : Nat)
    (row : top < bottom) (viewport : 0 < height) :
    reveal top bottom scroll height < bottom ∧
      top < reveal top bottom scroll height + height := by
  unfold reveal
  split
  · omega
  · split <;> omega

/-- 弹层开在锚的哪一侧：`preferred` 是部件自己的那一侧（composer 上方的列表朝上，表单里的组合框朝下），`other` 是对面。 -/
inductive Side where
  | preferred
  | other
  deriving DecidableEq, Repr

/-- D2：弹层打开时选一侧。`seen` 是锚是否落在裁剪它的那个盒子里（最近一个不让内容溢出的祖先，没有时是视口），`here` 与 `there` 是首选一侧与对面在那个盒子里各剩多高，`height` 是弹层高。
锚看不见时不换：一个卷到视口外的夹具没有「贴边」可言，量出来的空间说的是滚动位置而不是布局。被否决的做法是按两侧谁更宽来选，那会让一个两边都放得下的弹层随滚动位置来回换边；按视口而不按裁剪盒来量也被否决，因为设置页的组合框站在一个会滚动的面里，被裁掉的是那个面的边，不是窗口的边。 -/
def opensOn (seen : Bool) (here there height : Nat) : Side :=
  if seen ∧ here < height ∧ here < there then .other else .preferred

/-- 首选一侧放得下时，不论对面多宽、锚在哪里，弹层都开在首选一侧。 -/
theorem a_fitting_side_is_kept (seen : Bool) (here there height : Nat) (fits : height ≤ here) :
    opensOn seen here there height = .preferred := by
  unfold opensOn
  split
  · omega
  · rfl

/-- 换到对面时，对面比首选一侧宽，而首选一侧放不下。 -/
theorem a_flip_gains_room (seen : Bool) (here there height : Nat)
    (flipped : opensOn seen here there height = .other) :
    here < height ∧ here < there := by
  unfold opensOn at flipped
  split at flipped
  · omega
  · cases flipped

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
  | left => simpa only [press]
  | right => simpa only [press]
  | home => simp only [press]; omega
  | finish => simp only [press]; omega
  | tab => exact absurd rfl vertical.1
  | shiftTab => exact absurd rfl vertical.2

/-- 表的方向下，左右键只在当前行里走：游标仍在本行内。 -/
theorem a_side_step_stays_in_its_row (lists : Nat) (rows : Nat → Nat) (s : State)
    (inside : s.cursor < rows s.column) (key : Key) (side : key = .left ∨ key = .right) :
    (pressRow lists rows s key).cursor < rows (pressRow lists rows s key).column := by
  rcases side with rfl | rfl
  · exact clamp_stays (rows s.column) s.cursor .backward inside
  · exact clamp_stays (rows s.column) s.cursor .forward inside

/-- 表的方向下，上下键换行：游标复位到第 0 行，行号留在表内。 -/
theorem a_step_between_rows_starts_at_its_top (lists : Nat) (rows : Nat → Nat) (s : State)
    (h : 0 < lists) (key : Key) (step : key = .down ∨ key = .up) :
    (pressRow lists rows s key).cursor = 0 ∧ (pressRow lists rows s key).column < lists := by
  rcases step with rfl | rfl
  · exact ⟨rfl, wrap_stays lists s.column .forward h⟩
  · exact ⟨rfl, wrap_stays lists s.column .backward h⟩

theorem enter_applies_the_cursor_of_this_column (s : State) : applied s = (s.column, s.cursor) := rfl

end Client.Views.Parts.Popover
