-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# inspect/open：检视面的页签带

规定 `client/src/views/inspect/open.svelte.ts`（右侧的状态只有这一个家，§4-45）与检视面页签带的键（APG Tabs，键表在 `client/spec/Views/Workspace.lean` §7-11）。焦点还给打开者是 `client/spec/Views/Parts.lean` 的 `closeLayer`：这个模块是 §7-7 表里「状态模块」那一种实现。

每个打开的项一直挂载到关掉为止，所以至多 `KEPT`（8）项；多开一项就关掉最久没碰的那一项。

性质：

1. **打开一项之后至多 `KEPT` 项**（`at_most_kept_stay_open`），且刚打开的那一项在（`the_opened_item_stays`）。
2. **Delete 关掉焦点所在的页签，焦点落到它原来位置上的页签**，原来是最后一个时落到新的最后一个；落点在剩下的页签之内（`delete_lands_on_a_tab`）。
3. **←／→ 环绕**（同 `Client.Views.Parts.wrap`，`wrap_stays`）。
-/

namespace Client.Views.Inspect.Open

open Client.Views.Parts

/-- 至多挂载几项。 -/
def KEPT : Nat := 8

/-- 去掉第 `i` 项。 -/
def removeAt {α : Type} : List α → Nat → List α
  | [], _ => []
  | _ :: rest, 0 => rest
  | head :: rest, i + 1 => head :: removeAt rest i

theorem removeAt_length {α : Type} : ∀ (items : List α) (i : Nat), i < items.length →
    (removeAt items i).length + 1 = items.length := by
  intro items
  induction items with
  | nil => intro i h; simp at h
  | cons head rest ih =>
    intro i h
    cases i with
    | zero => simp [removeAt]
    | succ i =>
      simp only [removeAt, List.length_cons]
      have := ih i (by simp at h; omega)
      omega

/-- 多出一项时关掉哪一项由 `oldest` 选（最久没碰的那一项）；模型只要求它选的是一项。 -/
def evict {α : Type} (oldest : List α → Nat) (items : List α) : List α :=
  if KEPT < items.length then removeAt items (oldest items) else items

/-- 打开一项：已开着的移到最近，没开着的加上；然后至多留 `KEPT` 项。`oldest` 永不选刚打开的那一项（最后一项）。 -/
def openItem {α : Type} [DecidableEq α] (oldest : List α → Nat) (items : List α) (item : α) : List α :=
  evict oldest (items.filter (· ≠ item) ++ [item])

theorem at_most_kept_stay_open {α : Type} [DecidableEq α] (oldest : List α → Nat)
    (items : List α) (item : α) (kept : items.length ≤ KEPT)
    (picks : ∀ l : List α, oldest l < l.length) :
    (openItem oldest items item).length ≤ KEPT := by
  unfold openItem evict
  have grown : (items.filter (· ≠ item) ++ [item]).length ≤ KEPT + 1 := by
    simp only [List.length_append, List.length_cons, List.length_nil]
    have := List.length_filter_le (fun x => decide (x ≠ item)) items
    omega
  split
  · have := removeAt_length _ _ (picks (items.filter (· ≠ item) ++ [item]))
    omega
  · omega

theorem removeAt_keeps {α : Type} : ∀ (items : List α) (i j : Nat) (x : α),
    items[j]? = some x → i ≠ j → x ∈ removeAt items i := by
  intro items
  induction items with
  | nil => intro i j x h _; simp at h
  | cons head rest ih =>
    intro i j x h ne
    cases j with
    | zero =>
      cases i with
      | zero => exact absurd rfl ne
      | succ i => simp at h; subst h; simp [removeAt]
    | succ j =>
      cases i with
      | zero => simp [removeAt]; exact List.mem_of_getElem? h
      | succ i =>
        simp only [removeAt]
        exact List.mem_cons_of_mem _ (ih i j x (by simpa using h) (by omega))

theorem the_opened_item_stays {α : Type} [DecidableEq α] (oldest : List α → Nat)
    (items : List α) (item : α)
    (spares : ∀ l : List α, oldest l + 1 ≠ l.length) :
    item ∈ openItem oldest items item := by
  unfold openItem evict
  split
  · apply removeAt_keeps _ _ ((items.filter (· ≠ item) ++ [item]).length - 1)
    · simp
    · have := spares (items.filter (· ≠ item) ++ [item])
      omega
  · simp

/-- Delete 之后焦点落在第几个页签：`count` 是关掉之前的页签数。 -/
def afterDelete (count focused : Nat) : Nat := if focused + 1 < count then focused else focused - 1

theorem delete_lands_on_a_tab (count focused : Nat) (h : focused < count) (left : 2 ≤ count) :
    afterDelete count focused < count - 1 := by
  unfold afterDelete
  split <;> omega

example : afterDelete 3 2 = 1 := rfl

example : afterDelete 3 0 = 0 := rfl

end Client.Views.Inspect.Open
