-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# picker：设置行上的模型选择器

规定 `client/src/views/talk/picker.ts` 与 `picker_look.ts`（`docs/frontend-method.md` §7I，`client/Spec.lean` §4-60）。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`client/src/views/talk/picker.test.ts` 在 fixture 的每个可达状态上判读同样的答案。

令牌是一个控件，分三段：模型 · 供应商 · 档位。指针按在哪一段，弹层就把游标放进那一段的那一节；按键激活令牌（没有段在指针下）放进第一层。弹层用 `parts/popover.svelte` 的键表（`Popover.lean` 的 `press`）：↓／↑ 在一节之内走，Tab／Shift+Tab 换节，Enter 应用，Escape 关上并把焦点还给令牌；筛选框一直持焦点，键经 `bind` 交给键表。节的顺序：最近的组合、第一层、第二层、思考档带。

**哪一层在前由数量定**：按 `canonical` 合并后的模型数多于供应商数，第一层列供应商、第二层列它的模型；否则第一层列模型、第二层列它的供应商。顺序只是登记的 Endpoint 的函数，所以两次打开之间不会换。

**应用一行之后**：第一层或第二层的一行改选择、弹层留着，下面那一节跟着变；思考档或一条最近组合应用整组选择并关上；「更多…」把那一层全部展开；按在令牌与弹层之外关上，焦点落在按下的地方。

性质：

1. **一层至多画六行**：前五项与一行「更多…」（`a_list_draws_at_most_six_rows`）。
2. **窗口至多五项，打开时生效的那一项总在里面**（`the_window_holds_five`、`the_pinned_entry_is_shown`）。
3. **第二层的留白装得下每一项展开后的样子**（`any_second_level_fits_its_room`）。
4. **打开之后不打字能到的任何状态都不比打开时定下的高度高**，所以选模型或供应商不会让屏幕上已有的目标移动（`no_reachable_state_outgrows_the_open_height`）；「更多…」展开的那一层在自己的留白里滚动，不加高。
-/

namespace Client.Views.Picker

/-- 一层没打字时显示几项，其余由「更多…」或筛选得到。 -/
def shown : Nat := 5

/-- 一层有 `count` 项时画几行：至多五项，多出来时再加一行「更多…」。全部展开后它在同样的高度里滚动。 -/
def drawn (count : Nat) : Nat := if shown < count then shown + 1 else count

theorem a_list_draws_at_most_six_rows (count : Nat) : drawn count ≤ shown + 1 := by
  unfold drawn
  split <;> omega

/-- 一层显示的那几项：不超过五项时全部；否则前五项，打开时生效的那一项落在五项之后时，占第五个位置。 -/
def window {α : Type} [DecidableEq α] (entries : List α) (pinned : α) : List α :=
  if entries.length ≤ shown then entries
  else if pinned ∈ entries.drop shown then entries.take (shown - 1) ++ [pinned]
  else entries.take shown

theorem the_window_holds_five {α : Type} [DecidableEq α] (entries : List α) (pinned : α) :
    (window entries pinned).length ≤ shown := by
  unfold window
  split
  · assumption
  · split
    · simp only [List.length_append, List.length_take, List.length_singleton, shown]
      omega
    · simp only [List.length_take, shown]
      omega

theorem the_pinned_entry_is_shown {α : Type} [DecidableEq α] (entries : List α) (pinned : α)
    (listed : pinned ∈ entries) : pinned ∈ window entries pinned := by
  unfold window
  split
  · exact listed
  · split
    · exact List.mem_append_right _ (List.mem_singleton_self pinned)
    · rename_i late
      have whole := List.take_append_drop shown entries
      rw [← whole] at listed
      rcases List.mem_append.mp listed with early | dropped
      · exact early
      · exact absurd dropped late

/-- 第二层留几行：第一层每一项展开后画的行数里最大的那个。`counts` 是第一层每一项下面有几项。 -/
def room : List Nat → Nat
  | [] => 0
  | count :: rest => max (drawn count) (room rest)

/-- 第一层第 `entry` 项下面有几项；越过末尾的下标是一个空的第二层。 -/
def countAt : List Nat → Nat → Nat
  | [], _ => 0
  | count :: _, 0 => count
  | _ :: rest, entry + 1 => countAt rest entry

theorem any_second_level_fits_its_room (counts : List Nat) (entry : Nat) :
    drawn (countAt counts entry) ≤ room counts := by
  induction counts generalizing entry with
  | nil => simp only [countAt, room]; decide
  | cons head rest grows =>
    unfold room
    cases entry with
    | zero => exact Nat.le_max_left _ _
    | succ entry => exact Nat.le_trans (grows entry) (Nat.le_max_right _ _)

/-- 一个打开的选择器里不打字能改的东西：第一层展开的是哪一项，第二层生效的是哪一项，两层是否已全部展开。 -/
structure Opened where
  parent : Nat
  child : Nat
  wholeFirst : Bool
  wholeSecond : Bool
  deriving DecidableEq, Repr

/-- 不打字能做的一步。思考档与最近组合会关上弹层，所以不在这里。 -/
inductive Step where
  | expand (entry : Nat)
  | choose (entry : Nat)
  | moreFirst
  | moreSecond
  deriving DecidableEq, Repr

def step (s : Opened) : Step → Opened
  | .expand entry => { s with parent := entry }
  | .choose entry => { s with child := entry }
  | .moreFirst => { s with wholeFirst := true }
  | .moreSecond => { s with wholeSecond := true }

def run (s : Opened) : List Step → Opened
  | [] => s
  | next :: rest => run (step s next) rest

/-- 一个城市的选择器：第一层有几项，第一层每一项下面有几项，思考档行与它下面那一句占几行，以及哪一对（第一层，第二层）有思考档。 -/
structure Offers where
  first : Nat
  counts : List Nat
  band : Nat
  thinks : Nat → Nat → Bool

/-- 一个状态画出的行数。全部展开的一层仍只占 `drawn` 行，在里面滚动。 -/
def height (o : Offers) (s : Opened) : Nat :=
  drawn o.first + drawn (countAt o.counts s.parent) + (if o.thinks s.parent s.child then o.band else 0)

/-- 打开时定下的高度：第一层照画，第二层留 `room`，有任何一对有思考档就给思考档带留位。 -/
def reserved (o : Offers) (anyThinks : Bool) : Nat :=
  drawn o.first + room o.counts + (if anyThinks then o.band else 0)

theorem a_state_fits_the_reserve (o : Offers) (anyThinks : Bool)
    (covers : ∀ p c, o.thinks p c = true → anyThinks = true) (s : Opened) :
    height o s ≤ reserved o anyThinks := by
  unfold height reserved
  have second := any_second_level_fits_its_room o.counts s.parent
  have third : (if o.thinks s.parent s.child then o.band else 0) ≤ (if anyThinks then o.band else 0) := by
    cases hit : o.thinks s.parent s.child
    · simp only [Bool.false_eq_true, ↓reduceIte]
      omega
    · rw [covers _ _ hit]
      simp only [↓reduceIte, Nat.le_refl]
  omega

theorem no_reachable_state_outgrows_the_open_height (o : Offers) (anyThinks : Bool)
    (covers : ∀ p c, o.thinks p c = true → anyThinks = true) (start : Opened) (trace : List Step) :
    height o (run start trace) ≤ reserved o anyThinks :=
  a_state_fits_the_reserve o anyThinks covers (run start trace)

end Client.Views.Picker
