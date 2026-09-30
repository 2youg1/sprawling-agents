-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# The accounting thread and its one inbox.

Specifies `crates/sprawling/src/assembly/attending.rs`, the loop `attend`: the
one thread that writes a city's Ledger, and the three mouths it serves - a
lane's relay request, a run home from its lane, a command from the desk - plus
the desk closing. The Rust code is the authority on how the loop holds these
properties; this model is the authority on which properties must hold
(sprawling-SPEC.md 8-42-4).

Every mouth sends into one queue. The thread blocks on its first message
(`recv`), takes every message already behind it (`try_recv` until empty), and
serves that batch in arrival order. A deadline the schedule owes wakes it the
same way a message does.

Three properties, one theorem group each:

* **every message is served in finitely many steps** - a message waiting when
  the thread wakes is served by that wake, so one that has arrived is served by
  the next one;
* **append order is seq order** - the records the thread appends carry
  consecutive seqs from the first one, in the order they were appended;
* **no busy waiting** - with nothing waiting and no deadline due the thread
  does not wake, and each wake consumes work, so the wakes are bounded by the
  messages and deadlines that caused them.

What a served message does besides appending (settling a run, answering a
person) is the Rust code's and is not modelled; only how many records it
appends is.
-/

namespace Attending

/-- What reaches the accounting thread. Each variant carries how many records
serving it appends; closing appends none here, because what a closing city
writes is written by the messages it still serves. -/
inductive Message where
  | relay (records : Nat)
  | home (records : Nat)
  | command (records : Nat)
  | close
  deriving Repr, DecidableEq

/-- How many records serving a message appends. -/
def Message.records : Message → Nat
  | .relay n | .home n | .command n => n
  | .close => 0

/-- The thread's state: the queue in arrival order, the seq of every record
appended so far in append order, every message served in serving order, and
whether a schedule deadline has come due. -/
structure Loop where
  inbox : List Message
  ledger : List Nat
  served : List Message
  deadlineDue : Bool

/-- A sender puts a message at the back of the queue; it never reorders it. -/
def arrive (m : Message) (s : Loop) : Loop :=
  { s with inbox := s.inbox ++ [m] }

/-- The deadline the schedule owes comes due. -/
def due (s : Loop) : Loop :=
  { s with deadlineDue := true }

/-- Serving one message appends its records, each taking the next seq. -/
def serve (ledger : List Nat) (m : Message) : List Nat :=
  ledger ++ (List.range m.records).map (ledger.length + ·)

/-- The loop after one wake: the whole batch served in arrival order, the
deadline read. -/
def drained (s : Loop) : Loop :=
  { inbox := []
    ledger := s.inbox.foldl serve s.ledger
    served := s.served ++ s.inbox
    deadlineDue := false }

/-- One wake of the thread, or `none` while it sleeps in `recv`. -/
def wake (s : Loop) : Option Loop :=
  if s.inbox.isEmpty && !s.deadlineDue then none else some (drained s)

/-! ## Every message is served in finitely many steps -/

theorem wake_serves_every_waiting (s : Loop) (h : s.inbox ≠ []) :
    wake s = some (drained s) ∧ (drained s).inbox = [] ∧
      (drained s).served = s.served ++ s.inbox := by
  simp [wake, h, drained]

theorem arrived_is_served_by_the_next_wake (m : Message) (s : Loop) :
    ∃ s', wake (arrive m s) = some s' ∧ m ∈ s'.served := by
  refine ⟨drained (arrive m s), ?_, ?_⟩
  · exact (wake_serves_every_waiting (arrive m s) (by simp [arrive])).1
  · simp [drained, arrive]

/-! ## Append order is seq order -/

/-- The ledger's seqs are `0, 1, 2, …` in append order. -/
def Ordered (ledger : List Nat) : Prop :=
  ledger = List.range ledger.length

theorem serve_keeps_order {ledger : List Nat} (m : Message) (h : Ordered ledger) :
    Ordered (serve ledger m) := by
  unfold Ordered serve
  rw [List.length_append, List.length_map, List.length_range, List.range_add, ← h]

theorem batch_keeps_order (batch : List Message) {ledger : List Nat}
    (h : Ordered ledger) : Ordered (batch.foldl serve ledger) := by
  induction batch generalizing ledger with
  | nil => exact h
  | cons m rest ih => exact ih (serve_keeps_order m h)

theorem wake_keeps_order {s s' : Loop} (h : Ordered s.ledger) (w : wake s = some s') :
    Ordered s'.ledger := by
  unfold wake at w
  split at w
  · contradiction
  · cases w
    exact batch_keeps_order s.inbox h

/-! ## No busy waiting -/

/-- What a wake can consume: the messages waiting and a deadline come due. -/
def work (s : Loop) : Nat :=
  s.inbox.length + if s.deadlineDue then 1 else 0

theorem idle_does_not_wake (s : Loop) (empty : s.inbox = [])
    (quiet : s.deadlineDue = false) : wake s = none := by
  simp [wake, empty, quiet]

theorem every_wake_consumes_work {s s' : Loop} (w : wake s = some s') :
    work s' < work s := by
  unfold wake at w
  split at w
  · contradiction
  · rename_i busy
    cases w
    cases hi : s.inbox <;> cases hd : s.deadlineDue <;>
      simp_all [work, drained]

/-! ## A name is asked for off this thread, and the room is still the first write

A dispatch sent to a building with no room needs a name from the digest model,
a call that waits on a provider for seconds. The steps below are one such
dispatch in the order they happen (`assembly::dispatching::asking_name`):
agreeing writes nothing and is asked again once the name is home, because a
halt may have arrived meanwhile. -/

/-- One step of a dispatch that has to be named. -/
inductive DispatchStep where
  | agree
  | askName
  | openRoom
  | laterWrite
  deriving Repr, DecidableEq

/-- Whether the step puts anything on disk. -/
def DispatchStep.writes : DispatchStep → Bool
  | .openRoom | .laterWrite => true
  | .agree | .askName => false

/-- Whether the step runs on the accounting thread, where every relay request
waits for it. -/
def DispatchStep.onAccountingThread : DispatchStep → Bool
  | .askName => false
  | .agree | .openRoom | .laterWrite => true

def namedDispatch : List DispatchStep := [.agree, .askName, .agree, .openRoom, .laterWrite]

theorem opening_the_room_is_the_first_write :
    namedDispatch.find? DispatchStep.writes = some .openRoom := by decide

theorem nothing_is_written_before_the_name_is_home :
    (namedDispatch.takeWhile (· ≠ .askName)).all (!·.writes) := by decide

theorem the_naming_wait_is_off_the_accounting_thread :
    ∀ step ∈ namedDispatch, step.onAccountingThread = false → step = .askName := by decide

end Attending
