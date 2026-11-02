-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# session switcher：一棵会话树上的键盘契约

规定 `client/src/views/world/switcher.ts` 的 `answer`：zen 切换器的上下键只在可见行内移动，左右键只折叠或展开，Enter 才打开行，输入法组字中的键不改变状态。树的父子关系、默认折叠与环防护由 `core/session_tree.ts` 的判定与 `session_tree.test.ts` 承担；本模型只证明收键状态机。

打开之后仍是 `client/spec/Views/Talk/Branching.lean` 的「先分支、再前往」，这里不复制那条规则。
-/

namespace Client.Views.SessionSwitch

inductive Key where
  | up | down | left | right | enter | composing | other
  deriving DecidableEq, Repr

structure Node where
  key : Nat
  children : Bool
  deriving DecidableEq, Repr

structure State where
  cursor : Nat
  collapsed : List Nat
  deriving DecidableEq, Repr

def clamp (n : Nat) : Nat → Nat
  | 0 => 0
  | n' + 1 => min n n'

def fold (keys : List Nat) (key : Nat) : List Nat :=
  if keys.contains key then keys.filter (· ≠ key) else key :: keys

def openKey (keys : List Nat) (key : Nat) : List Nat := keys.filter (· ≠ key)

def nodeAt (nodes : List Node) (i : Nat) : Option Node :=
  match nodes with
  | [] => none
  | node :: rest => if i = 0 then some node else nodeAt rest (i - 1)

/-- 返回新状态与这一步是否请求打开一行。 -/
def answer (nodes : List Node) (s : State) : Key → State × Bool
  | .up => ({ s with cursor := s.cursor.pred }, false)
  | .down => ({ s with cursor := clamp (s.cursor + 1) nodes.length }, false)
  | .left =>
    (match nodeAt nodes s.cursor with
      | some node => if node.children then ({ s with collapsed := fold s.collapsed node.key }, false) else (s, false)
      | none => (s, false))
  | .right =>
    (match nodeAt nodes s.cursor with
      | some node => if node.children then ({ s with collapsed := openKey s.collapsed node.key }, false) else (s, false)
      | none => (s, false))
  | .enter => (s, nodeAt nodes s.cursor |>.isSome)
  | .composing | .other => (s, false)

theorem composing_changes_nothing (nodes : List Node) (s : State) :
    answer nodes s .composing = (s, false) := rfl

theorem arrows_never_open (nodes : List Node) (s : State) (key : Key)
    (h : key = .up ∨ key = .down ∨ key = .left ∨ key = .right) :
    (answer nodes s key).2 = false := by
  rcases h with rfl | rfl | rfl | rfl
  · rfl
  · rfl
  · simp only [answer]
    split
    · split <;> rfl
    · rfl
  · simp only [answer]
    split
    · split <;> rfl
    · rfl

theorem only_enter_opens (nodes : List Node) (s : State) (key : Key)
    (h : (answer nodes s key).2 = true) : key = .enter := by
  cases key <;> simp [answer] at h ⊢
  · split at h
    · split at h <;> simp at h
    · simp at h
  · split at h
    · split at h <;> simp at h
    · simp at h

theorem clamp_le (n bound : Nat) : clamp n bound ≤ bound := by
  cases bound <;> simp [clamp] <;> omega

theorem cursor_stays_inside (nodes : List Node) (s : State) (key : Key)
    (before : s.cursor ≤ nodes.length) : (answer nodes s key).1.cursor ≤ nodes.length := by
  cases key <;> simp [answer]
  · omega
  · exact clamp_le _ _
  · split
    · split <;> simpa using before
    · exact before
  · split
    · split <;> simpa using before
    · exact before
  all_goals exact before

end Client.Views.SessionSwitch
