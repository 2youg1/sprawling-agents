-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# decide：请决定卡的三个键

规定 `client/src/views/parts/decide.svelte` 的键（`role="group"`，y／e／n），键表在 `client/spec/Views/Workspace.lean` §7-11，卡的种类与答复见 `client/Spec.lean` §7C 与 §4-55。一张卡至多三个答复，各在一个键上；卡上没有的答复、说明了为什么按不动的答复，按键落空；焦点在文本框里时字母是字，不是答复。

性质：

1. **一个门在等人亲自动手时只有「知道了」**（`a_gate_only_takes_no`）：城那边没有可替人发的帧。
2. **基线过期的提案只能拒绝**（`a_stale_proposal_only_refuses`）。
3. **文本框里的 y 是字**（`a_letter_in_a_field_is_a_letter`）。
4. **按键落下的答复一定是卡上给出、且按得动的那一个**（`a_key_answers_only_what_the_card_offers`）。
-/

namespace Client.Views.Parts.Decide

/-- 卡的种类；提案带它的基线是否已过期。 -/
inductive Kind where
  | question
  | ask
  | proposal (stale : Bool)
  deriving DecidableEq, Repr

/-- 三个答复：y 同意、e 改后同意、n 不。 -/
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

/-- 一个字母键落下的答复：焦点在文本框里、卡上没有、或这一答复按不动（`blocked`）时落空。 -/
def keyed (kind : Kind) (inField : Bool) (blocked : Reply → Bool) (reply : Reply) : Option Reply :=
  if inField then none
  else if reply ∈ offered kind ∧ blocked reply = false then some reply
  else none

theorem a_gate_only_takes_no (blocked : Reply → Bool) (reply : Reply) (h : reply ≠ .no) :
    keyed .ask false blocked reply = none := by
  cases reply <;> simp_all [keyed, offered]

theorem a_stale_proposal_only_refuses (blocked : Reply → Bool) (reply : Reply) (h : reply ≠ .no) :
    keyed (.proposal true) false blocked reply = none := by
  cases reply <;> simp_all [keyed, offered]

theorem a_letter_in_a_field_is_a_letter (kind : Kind) (blocked : Reply → Bool) (reply : Reply) :
    keyed kind true blocked reply = none := by
  simp [keyed]

theorem a_key_answers_only_what_the_card_offers (kind : Kind) (inField : Bool)
    (blocked : Reply → Bool) (reply answer : Reply)
    (h : keyed kind inField blocked reply = some answer) :
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
