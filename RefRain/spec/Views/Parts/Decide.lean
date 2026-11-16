-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# decide：请决定卡的三个键

规定 `client/src/views/parts/decide.svelte` 的键（`role="group"`，Accel-Shift-Y／E／X），键表在 `client/spec/Views/Workspace.lean` §7-11，卡的种类与答复见 `client/Spec.lean` §7C 与 §4-55。一张卡至多三个答复，各在一个键上；卡上没有的答复、说明了为什么按不动的答复，按键落空；三个答复键都带 accel（`client/Spec.lean` D92），不带 accel 的一下按键——焦点在文本框里也好、停在卡上也好——不是答复，所以误打的一个字母不会替人作决定。

性质：

1. **一个门在等人亲自动手时只有「知道了」**（`a_gate_only_takes_no`）：城那边没有可替人发的帧。
2. **基线过期的提案只能拒绝**（`a_stale_proposal_only_refuses`）。
3. **不带 accel 的一下按键不是答复**（`a_press_without_the_accelerator_is_no_answer`）；`client/src/core/keys.test.ts` 的「every action but focusing the message box holds the accelerator」判默认表确实如此。
4. **按键落下的答复一定是卡上给出、且按得动的那一个**（`a_key_answers_only_what_the_card_offers`）。
-/

namespace Client.Views.Parts.Decide

/-- 卡的种类；提案带它的基线是否已过期。 -/
inductive Kind where
  | question
  | ask
  | proposal (stale : Bool)
  deriving DecidableEq, Repr

/-- 三个答复：同意、改后同意、不同意。 -/
inductive Reply where
  | yes
  | edit
  | no
  deriving DecidableEq, Repr

/-- 每种卡给出的答复。 -/
def offered : Kind → List Reply
  | .question => [.yes, .no]
  | .ask => [.no]
  | .proposal false => [.yes, .edit, .no]
  | .proposal true => [.no]

/-- 一下按键落下的答复：不带 accel（`accel = false`）、卡上没有、或这一答复按不动（`blocked`）时落空。 -/
def keyed (kind : Kind) (accel : Bool) (blocked : Reply → Bool) (reply : Reply) : Option Reply :=
  if accel = false then none
  else if reply ∈ offered kind ∧ blocked reply = false then some reply
  else none

theorem a_gate_only_takes_no (blocked : Reply → Bool) (reply : Reply) (h : reply ≠ .no) :
    keyed .ask true blocked reply = none := by
  cases reply <;> simp_all [keyed, offered]

theorem a_stale_proposal_only_refuses (blocked : Reply → Bool) (reply : Reply) (h : reply ≠ .no) :
    keyed (.proposal true) true blocked reply = none := by
  cases reply <;> simp_all [keyed, offered]

theorem a_press_without_the_accelerator_is_no_answer (kind : Kind) (blocked : Reply → Bool) (reply : Reply) :
    keyed kind false blocked reply = none := by
  simp [keyed]

theorem a_key_answers_only_what_the_card_offers (kind : Kind) (accel : Bool)
    (blocked : Reply → Bool) (reply answer : Reply)
    (h : keyed kind accel blocked reply = some answer) :
    answer = reply ∧ answer ∈ offered kind ∧ blocked answer = false := by
  unfold keyed at h
  split at h
  · cases h
  · split at h
    · cases h
      rename_i held
      exact ⟨rfl, held.1, held.2⟩
    · cases h

end Client.Views.Parts.Decide
