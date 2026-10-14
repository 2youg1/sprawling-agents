-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# combobox：一层 listbox 弹层

规定 `client/src/views/parts/combobox.svelte`（APG Combobox，listbox 弹层），键表在 `client/spec/Views/Parts.lean` §7-5。

状态是：弹层开不开、游标在第几行、过滤词、焦点在哪，以及**生效的值**（`chosen`，`aria-selected` 读它）。游标由持焦元素的 `aria-activedescendant` 承担，从不写进 `aria-selected`。

性质：

1. **游标走动不改生效的值**（`the_cursor_is_not_the_selection`）：`aria-selected` 在 combobox 与 popover 里说同一件事，「这是当前生效的值」。
2. **游标钳在列表两端**（`a_move_stays_inside`）。
3. **打一个字过滤，游标复位到第 0 行**（`typing_resets_the_cursor`）。
4. **Enter 采纳游标行、关上弹层、焦点回触发器**（`enter_adopts_the_cursor`）。
5. **Escape 关上弹层、清空过滤词、焦点回触发器、不改生效的值**（`escape_closes_and_keeps_the_value`）。
6. **Tab 关上弹层并让焦点正常离开**（`tab_lets_the_focus_leave`）。
7. **输入法组字时按下的键归输入法**（`only_settled_keys_act`）：一串按键里夹着多少组字中的键，结果都与只按其中已落定的键相同，所以确认候选的 Enter 不采纳游标行，挑候选的方向键不动游标，放弃组字的 Escape 不关弹层。
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

/-! D99 输入法组字时，文本框里的每一个键都归输入法，键表一律不答

- **决策**：键事件带着 `isComposing` 为真时，组合框的键表（`parts/combobox.ts` 的 `answer`，`talk/pill.ts` 的菜单读同一张表）答「不接」：不移游标、不采纳、不关弹层、不 `preventDefault`。规则写在键表函数里，不写在调用点：一个读这张表的新座位因此不必记得再查一遍。palette 与文件查找共用的 `views/cursor.ts` 是另一张表，开头查同一个字段；composer（`talk/composer.ts` 的 `boxKey`）与模型选择器的筛选框（`talk/picker_look.ts`）原本就先查它。
- **理由**：组字时 Enter 确认候选，方向键在候选之间走，Escape 放弃这次组字；三个键都已经有输入法给的意思。只放过 Enter 时，人用方向键挑一个候选，游标跟着在列表里走；用 Escape 放弃一段拼音，弹层连同过滤词一起关掉。读中文界面的人正是用输入法打字的人。
- **被击败的备选**：①只在 Enter 上查 `isComposing`——见理由；②每个调用点在调用键表之前各查一次（composer 与选择器今天的写法）——组合框与芯片两个座位已经各写过一份键表，规则再分到每个调用点，下一个座位漏掉它时没有任何东西会变红；③听 `compositionstart`／`compositionend` 自己记一个状态——平台已经在每个键事件上给出这个事实，第二份状态只会在两个事件之间与它不一致。
- **重开参数**：某个引擎确认候选的那一下 keydown 给出 `isComposing` 为假（有报告说 WebKit 在 `compositionend` 之后才发这个 keydown，`keyCode` 为 229）；那时这个判定连同 composer 与选择器的一起改读同一个更宽的事实。
-/

/-- 一次按键：落定的键，或输入法组字时按下的键（`isComposing` 为真）。 -/
inductive Press where
  | settled (key : Key)
  | composing (key : Key)
  deriving DecidableEq, Repr

/-- 一次按键之后的状态：组字中的键不改任何东西。 -/
def step (rows : List Char → Nat) (s : State) : Press → State
  | .settled key => press rows s key
  | .composing _ => s

/-- 一串按键里已落定的那些键，按原来的次序。 -/
def settledKeys : List Press → List Key :=
  List.filterMap fun
    | .settled key => some key
    | .composing _ => none

theorem only_settled_keys_act (rows : List Char → Nat) (trace : List Press) :
    ∀ s : State, trace.foldl (step rows) s = (settledKeys trace).foldl (press rows) s := by
  induction trace with
  | nil => intro s; rfl
  | cons first rest ih =>
    intro s
    cases first with
    | settled key => exact ih (press rows s key)
    | composing key => exact ih s

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
