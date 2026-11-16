-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# tabs：自动激活的页签

规定 `client/src/views/parts/tabs.svelte`（APG Tabs，自动激活），键表在 `client/spec/Views/Parts.lean` §7-4。自动激活是 APG 对「面板内容已在本地、切换无可察延迟」的推荐读法：焦点到哪个页签，哪个页签就是当前的，所以状态只有一个下标，「选中」与「聚焦」不可能分开。

性质：

1. **每个键都留在页签之内**（`a_key_stays_inside`）。
2. **→ 与 ← 环绕，且互为逆**（`left_undoes_right`）。
3. **Tab 序列里只有当前那个页签**：`tabindex` 恰对当前页签为 `0`（`only_the_current_tab_is_a_stop`）。
4. **Space 与 Enter 不额外做事**（`space_and_enter_change_nothing`）：切换已经跟随焦点。
-/

namespace Client.Views.Parts.Tabs

open Client.Views.Parts

/-- 页签带收的键。 -/
inductive Key where
  | right
  | left
  | home
  | finish
  | space
  | enter
  deriving DecidableEq, Repr

/-- 一个键之后的当前页签。 -/
def press (total current : Nat) : Key → Nat
  | .right => wrap total current .forward
  | .left => wrap total current .backward
  | .home => 0
  | .finish => total - 1
  | .space => current
  | .enter => current

/-- 一个页签的 `tabindex`：当前的是 `0`，其余是 `-1`。 -/
def tabindex (current tab : Nat) : Int := if tab = current then 0 else -1

theorem a_key_stays_inside (total current : Nat) (key : Key) (h : current < total) :
    press total current key < total := by
  cases key with
  | right => exact wrap_stays total current .forward (by omega)
  | left => exact wrap_stays total current .backward (by omega)
  | home => simp only [press]; omega
  | finish => simp only [press]; omega
  | space => exact h
  | enter => exact h

theorem left_undoes_right (total current : Nat) (h : current < total) :
    press total (press total current .right) .left = current :=
  wrap_back_undoes_forward total current h

theorem only_the_current_tab_is_a_stop (current tab : Nat) :
    tabindex current tab = 0 ↔ tab = current := by
  unfold tabindex
  split <;> simp_all

theorem space_and_enter_change_nothing (total current : Nat) :
    press total current .space = current ∧ press total current .enter = current := ⟨rfl, rfl⟩

end Client.Views.Parts.Tabs
