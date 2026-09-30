-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# A run whose resident is an official harness.

Specifies the harness run the dispatch path starts for a room whose resident
is one of the five official harnesses (sprawling-SPEC.md section 8-4e), over the
ACP session in `crates/protocol/src/harness/session.rs` (agent_protocols-SPEC.md
section 8-19). The Rust code is the authority on how these properties hold;
this model is the authority on which properties must hold.

A harness runs its own tools. The city cannot admit or refuse what it does; it
can only record what the harness chooses to report. So a harness run keeps a
weaker rule than ARCHITECTURE.md section 5 step 4 ("every effect becomes an
event first"): what the city itself decides - that the run started, that a
halt was turned into a cancel, what the harness answered to the city's own
prompt, how the run ended - is admitted history, and what the harness says it
did along the way is a report, booked after the harness said it and never read
back as a decision.

The run is a machine over three inputs, in the order the city observes them: a
report the harness sent, a halt of a scope that holds the room, and the answer
to the prompt carrying a stop reason. It emits four outputs: a report booked,
the cancel sent to the harness, the answer to the prompt recorded, and the run
frozen with its stop reason.

Three properties, one theorem group each:

* **a report is never admitted history** - the admitted records of a harness
  run are its start, the answer to its prompt and its freeze, whatever and
  however many reports arrive between them;
* **a halt is a cancel before anything else** - once a halt is observed, the
  next thing the run emits is the cancel, so no report is booked between the
  halt and the cancel, and a second halt sends nothing;
* **a frozen run is history** - after the stop reason nothing is emitted: no
  report is booked and no cancel is sent to a session that has ended.
-/

namespace HarnessRun

/-- Why the harness ended its turn, as ACP names it. -/
inductive Stop where
  | endTurn
  | maxTokens
  | maxTurnRequests
  | refusal
  | cancelled
  deriving Repr, DecidableEq

/-- What the city observes while the harness works, in the order observed.
A report carries what the harness said, abstracted to a number. -/
inductive Input where
  | report (said : Nat)
  | halt
  | stop (why : Stop)
  deriving Repr, DecidableEq

/-- What the run does in answer. -/
inductive Output where
  | book (said : Nat)
  | cancel
  | answer (why : Stop)
  | freeze (why : Stop)
  deriving Repr, DecidableEq

/-- Whether a halt has been turned into a cancel, and whether the run froze. -/
structure State where
  cancelled : Bool
  frozen : Bool
  deriving Repr, DecidableEq

/-- A run that has just started: nothing cancelled, nothing frozen. -/
def State.fresh : State := ⟨false, false⟩

/-- One observation and what the run emits for it. -/
def step (s : State) : Input → State × List Output
  | .report said => if s.frozen then (s, []) else (s, [.book said])
  | .halt =>
    if s.frozen || s.cancelled then (s, []) else ({ s with cancelled := true }, [.cancel])
  | .stop why =>
    if s.frozen then (s, []) else ({ s with frozen := true }, [.answer why, .freeze why])

/-- Everything a run emits for a sequence of observations. -/
def run : State → List Input → List Output
  | _, [] => []
  | s, i :: rest => (step s i).2 ++ run (step s i).1 rest

/-- The state after a sequence of observations. -/
def after : State → List Input → State
  | s, [] => s
  | s, i :: rest => after (step s i).1 rest

theorem run_append (s : State) (xs ys : List Input) :
    run s (xs ++ ys) = run s xs ++ run (after s xs) ys := by
  induction xs generalizing s with
  | nil => rfl
  | cons i rest ih => simp [run, after, ih, List.append_assoc]

/-! ## A report is never admitted history -/

/-- A Ledger record a harness run writes: admitted when the city decided it,
reported when the harness said it. -/
inductive Record where
  | started
  | reported (said : Nat)
  | cancelSent
  | answered (why : Stop)
  | frozen (why : Stop)
  deriving Repr, DecidableEq

/-- The record each output becomes. -/
def Output.record : Output → Record
  | .book said => .reported said
  | .cancel => .cancelSent
  | .answer why => .answered why
  | .freeze why => .frozen why

/-- The records of one run: its start, then one record per output. -/
def ledger (s : State) (inputs : List Input) : List Record :=
  .started :: (run s inputs).map Output.record

/-- What the city decided, read from the records: every record but a report.
Every fold that answers a question about what the city did reads this, and a
report is read only where the question is what the harness said. -/
def admitted : List Record → List Record
  | [] => []
  | .reported _ :: rest => admitted rest
  | r :: rest => r :: admitted rest

/-- Only reports. -/
def reportsOnly (inputs : List Input) : Prop :=
  ∀ i ∈ inputs, ∃ said, i = .report said

theorem reports_emit_only_bookings (s : State) (inputs : List Input)
    (h : reportsOnly inputs) :
    admitted ((run s inputs).map Output.record) = [] := by
  induction inputs generalizing s with
  | nil => rfl
  | cons i rest ih =>
    obtain ⟨said, rfl⟩ := h i (List.mem_cons_self ..)
    have hrest : reportsOnly rest := fun j hj => h j (List.mem_cons_of_mem _ hj)
    cases hf : s.frozen <;> simp [run, step, hf, admitted, Output.record, ih _ hrest]

theorem reports_leave_the_state (s : State) (inputs : List Input)
    (h : reportsOnly inputs) : after s inputs = s := by
  induction inputs generalizing s with
  | nil => rfl
  | cons i rest ih =>
    obtain ⟨said, rfl⟩ := h i (List.mem_cons_self ..)
    have hrest : reportsOnly rest := fun j hj => h j (List.mem_cons_of_mem _ hj)
    cases hf : s.frozen <;> simp [after, step, hf, ih _ hrest]

theorem admitted_append (xs ys : List Record) :
    admitted (xs ++ ys) = admitted xs ++ admitted ys := by
  induction xs with
  | nil => rfl
  | cons r rest ih => cases r <;> simp [admitted, ih]

/-- Any number of reports, then the stop reason: the admitted history is the
start, the answer and the freeze, whatever the harness reported. -/
theorem a_report_is_never_admitted (reports : List Input) (why : Stop)
    (h : reportsOnly reports) :
    admitted (ledger .fresh (reports ++ [.stop why])) =
      [.started, .answered why, .frozen why] := by
  have stays := reports_leave_the_state State.fresh reports h
  have booked := reports_emit_only_bookings State.fresh reports h
  simp only [ledger, run_append, List.map_append, stays]
  simp only [admitted, admitted_append, booked]
  simp [admitted, run, step, State.fresh, Output.record]

/-! ## A halt is a cancel before anything else -/

/-- Once a halt is observed on a live run, the next thing emitted is the
cancel: the reports observed after it are booked behind it. -/
theorem a_halt_is_a_cancel_before_anything_else (s : State) (before later : List Input)
    (live : (after s before).frozen = false) (first : (after s before).cancelled = false) :
    run s (before ++ .halt :: later) =
      run s before ++ .cancel :: run { after s before with cancelled := true } later := by
  rw [run_append]
  simp [run, step, live, first]

/-- A second halt sends nothing: the harness is cancelled once. -/
theorem a_second_halt_sends_nothing (s : State) (later : List Input)
    (already : s.cancelled = true) :
    run s (.halt :: later) = run s later := by
  simp [run, step, already]

/-! ## A frozen run is history -/

theorem a_frozen_run_emits_nothing (s : State) (later : List Input)
    (done : s.frozen = true) : run s later = [] := by
  induction later with
  | nil => rfl
  | cons i rest ih => cases i <;> simp [run, step, done, ih]

/-- The run that answered its stop reason records the answer, freezes, and
emits nothing for anything observed after it: no report, no cancel. -/
theorem nothing_follows_the_stop_reason (s : State) (before later : List Input) (why : Stop)
    (live : (after s before).frozen = false) :
    run s (before ++ .stop why :: later) = run s before ++ [.answer why, .freeze why] := by
  rw [run_append]
  simp [run, step, live]
  exact a_frozen_run_emits_nothing _ _ rfl

end HarnessRun
