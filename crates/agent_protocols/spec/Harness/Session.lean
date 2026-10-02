-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# A run whose resident is an official harness.

Specifies the harness run the dispatch path starts for a room whose resident
is one of the five official harnesses (`crates/sprawling/Spec.lean` §8-4e and
8-124). Three Rust modules hold it between them, and the Rust code is the
authority on how these properties hold; this model is the authority on which
properties must hold:

* `crates/agent_protocols/src/harness/session.rs` - the ACP session: a halt
  becomes `session/cancel`, a report is handed on, a stop reason ends the turn
  (`crates/agent_protocols/Spec.lean` section 8-19);
* `crates/runtime/src/run/harness.rs` - the lines the run writes: its opening,
  each report, the cancel, the checkpoint, the answer and the freeze, and which
  `kernel::Completion` the answer freezes as (`crates/runtime/Spec.lean` §8-52);
* `crates/accounting/src/worker/driving/harness.rs` - the drive that joins the
  two in a lane: what counts as a cut, and the tree the checkpoint commits.

A harness runs its own tools. The city cannot admit or refuse what it does; it
can only record what the harness chooses to report. So a harness run keeps a
weaker rule than ARCHITECTURE.md section 5 step 4 ("every effect becomes an
event first"): what the city itself decides - that the run started, that a
halt or the building's wall-clock ceiling was turned into a cancel, what the
harness's tree held when it stopped, what the harness answered to the city's
own prompt, how the run ended - is admitted history, and what the harness says
it did along the way is a report, booked after the harness said it and never
read back as a decision.

The run is a machine over four inputs, in the order the city observes them: a
report the harness sent, a cut (a halt of a scope that holds the room, or the
wall-clock ceiling), the answer to the prompt carrying a stop reason, and the
loss of the session before any answer. It emits five outputs: a report
booked, the cancel sent to the harness, the tree committed, the answer
recorded, and the run frozen with its ending.

Six properties, one theorem group each:

* **a report is never admitted history** - the admitted records of a harness
  run are its start, the checkpoint, the answer to its prompt and its freeze,
  whatever and however many reports arrive between them;
* **a cut is a cancel before anything else** - once a cut is observed, the
  next thing the run emits is the cancel, so no report is booked between the
  cut and the cancel, and a second cut sends nothing;
* **the tree is committed before the answer, and a frozen run is history** -
  the stop reason emits the checkpoint, the answer and the freeze in that
  order, and after it nothing is emitted: no report, no cancel;
* **a lost session freezes cancelled with no answer** - the city records no
  answer the harness never gave;
* **done needs an end of turn that said something** - every other stop, and an
  end of turn with no words, freezes as limit or cancelled, and the ceiling
  never freezes as cancelled;
* **only admitted history is evidence** - the one record a claim of done may
  cite is the answer to the city's prompt, and no report is ever admitted, so
  a report is never evidence.
-/

namespace HarnessRun

/-- Why the harness ended its turn, as ACP names it
(`kernel::event::record::HarnessStop`). -/
inductive Stop where
  | endTurn
  | maxTokens
  | maxTurnRequests
  | refusal
  | cancelled
  deriving Repr, DecidableEq

/-- Why the city cut a turn short (`runtime::run::harness::Cut`): a halt of a
scope that holds the room - the person's cancel of the run is one - or the
wall-clock ceiling the building's rules set. -/
inductive Cut where
  | halt
  | deadline
  deriving Repr, DecidableEq

/-- How a run ends, as `kernel::Completion` names it. -/
inductive Ending where
  | done
  | limit
  | cancelled
  deriving Repr, DecidableEq

/-- What the city observes while the harness works, in the order observed.
A report carries what the harness said, abstracted to a number; a stop carries
whether the answer said anything. -/
inductive Input where
  | report (said : Nat)
  | cut (why : Cut)
  | stop (why : Stop) (spoke : Bool)
  | lost
  deriving Repr, DecidableEq

/-- What the run does in answer. -/
inductive Output where
  | book (said : Nat)
  | cancel
  | checkpoint
  | answer (why : Stop)
  | freeze (how : Ending)
  deriving Repr, DecidableEq

/-- Which cut the city made, if any, and whether the run froze. -/
structure State where
  cut : Option Cut
  frozen : Bool
  deriving Repr, DecidableEq

/-- A run that has just started: nothing cut, nothing frozen. -/
def State.fresh : State := ⟨none, false⟩

/-- How an answer ends the run (`crates/sprawling/Spec.lean` §8-4e rules 3 and 8, 8-124).
An end of turn that said something is done; one that said nothing is limit,
the reading `runtime::run::lifecycle::concluded` gives an empty model reply. A
turn the ceiling cancelled ended against something, so it is limit; a turn a
halt cancelled is cancelled. The three stops that hit something are limit. -/
def ending (why : Stop) (spoke : Bool) (cut : Option Cut) : Ending :=
  match why with
  | .endTurn => if spoke then .done else .limit
  | .cancelled => if cut = some .deadline then .limit else .cancelled
  | .maxTokens | .maxTurnRequests | .refusal => .limit

/-- One observation and what the run emits for it. -/
def step (s : State) : Input → State × List Output
  | .report said => if s.frozen then (s, []) else (s, [.book said])
  | .cut why =>
    if s.frozen || s.cut.isSome then (s, []) else ({ s with cut := some why }, [.cancel])
  | .stop why spoke =>
    if s.frozen then (s, [])
    else ({ s with frozen := true }, [.checkpoint, .answer why, .freeze (ending why spoke s.cut)])
  | .lost => if s.frozen then (s, []) else ({ s with frozen := true }, [.freeze .cancelled])

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
  | checkpointed
  | answered (why : Stop)
  | frozen (how : Ending)
  deriving Repr, DecidableEq

/-- The record each output becomes. -/
def Output.record : Output → Record
  | .book said => .reported said
  | .cancel => .cancelSent
  | .checkpoint => .checkpointed
  | .answer why => .answered why
  | .freeze how => .frozen how

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
start, the checkpoint, the answer and the freeze, whatever the harness
reported. -/
theorem a_report_is_never_admitted (reports : List Input) (why : Stop) (spoke : Bool)
    (h : reportsOnly reports) :
    admitted (ledger .fresh (reports ++ [.stop why spoke])) =
      [.started, .checkpointed, .answered why, .frozen (ending why spoke none)] := by
  have stays := reports_leave_the_state State.fresh reports h
  have booked := reports_emit_only_bookings State.fresh reports h
  simp only [ledger, run_append, List.map_append, stays]
  simp only [admitted, admitted_append, booked]
  simp [admitted, run, step, State.fresh, Output.record]

/-! ## A cut is a cancel before anything else -/

/-- Once a cut is observed on a live run, the next thing emitted is the
cancel: the reports observed after it are booked behind it. A halt and the
ceiling take this one path. -/
theorem a_halt_is_a_cancel_before_anything_else (s : State) (before later : List Input)
    (why : Cut) (live : (after s before).frozen = false)
    (first : (after s before).cut = none) :
    run s (before ++ .cut why :: later) =
      run s before ++ .cancel :: run { after s before with cut := some why } later := by
  rw [run_append]
  simp [run, step, live, first]

/-- A second cut sends nothing: the harness is cancelled once, and the first
cut is the one the ending reads. -/
theorem a_second_halt_sends_nothing (s : State) (later : List Input) (why earlier : Cut)
    (already : s.cut = some earlier) :
    run s (.cut why :: later) = run s later := by
  simp [run, step, already]

/-! ## The tree is committed before the answer, and a frozen run is history -/

theorem a_frozen_run_emits_nothing (s : State) (later : List Input)
    (done : s.frozen = true) : run s later = [] := by
  induction later with
  | nil => rfl
  | cons i rest ih => cases i <;> simp [run, step, done, ih]

/-- The run that answered its stop reason commits the tree, records the
answer, freezes with the ending the answer and the first cut decide, and
emits nothing for anything observed after it: no report, no cancel. -/
theorem nothing_follows_the_stop_reason (s : State) (before later : List Input) (why : Stop)
    (spoke : Bool) (live : (after s before).frozen = false) :
    run s (before ++ .stop why spoke :: later) =
      run s before ++
        [.checkpoint, .answer why, .freeze (ending why spoke (after s before).cut)] := by
  rw [run_append]
  simp [run, step, live]
  exact a_frozen_run_emits_nothing _ _ rfl

/-! ## A lost session freezes cancelled with no answer -/

/-- A session that ended without a stop reason leaves no answer to record:
the run freezes cancelled, and nothing follows. -/
theorem a_lost_session_freezes_cancelled_with_no_answer (s : State)
    (before later : List Input) (live : (after s before).frozen = false) :
    run s (before ++ .lost :: later) = run s before ++ [.freeze .cancelled] := by
  rw [run_append]
  simp [run, step, live]
  exact a_frozen_run_emits_nothing _ _ rfl

/-! ## Done needs an end of turn that said something -/

theorem done_needs_an_end_turn_that_spoke {why : Stop} {spoke : Bool} {cut : Option Cut}
    (h : ending why spoke cut = .done) : why = .endTurn ∧ spoke = true := by
  cases why <;> cases spoke <;> simp [ending] at h ⊢
  all_goals (split at h <;> simp at h)

/-- The ceiling ends a run that hit something, whatever the harness answered,
so it never reads as a run a person cancelled. -/
theorem a_deadline_never_freezes_cancelled (why : Stop) (spoke : Bool) :
    ending why spoke (some .deadline) ≠ .cancelled := by
  cases why <;> cases spoke <;> simp [ending]

/-! ## Only an admitted record is cited as evidence -/

/-- The records a claim that the run is done may cite (`crates/kernel/Spec.lean` §8-20, `kernel::completion::CITABLE`): the answer to the city's prompt. A report
is what the harness said, and the city never decided it. -/
def Record.citable : Record → Bool
  | .answered _ => true
  | .started | .reported _ | .cancelSent | .checkpointed | .frozen _ => false

/-- No report survives `admitted`, whatever else the records hold. -/
theorem no_report_is_admitted (said : Nat) :
    ∀ records : List Record, Record.reported said ∉ admitted records
  | [] => by simp [admitted]
  | r :: rest => by
    have later := no_report_is_admitted said rest
    cases r <;> simp [admitted, later]

/-- A record that is not a report survives `admitted`. -/
theorem kept_by_admitted {r : Record} (notReport : ∀ said, r ≠ .reported said) :
    ∀ {records : List Record}, r ∈ records → r ∈ admitted records
  | _ :: _, .head _ => by
    cases r with
    | reported said => exact absurd rfl (notReport said)
    | started => simp [admitted]
    | cancelSent => simp [admitted]
    | checkpointed => simp [admitted]
    | answered _ => simp [admitted]
    | frozen _ => simp [admitted]
  | x :: _, .tail _ later => by
    have kept := kept_by_admitted notReport later
    cases x <;> simp [admitted, kept]

/-- Whatever a run observed, a record its claim of done may cite is admitted
history, so a report is never the evidence of done. -/
theorem a_cited_record_is_admitted (s : State) (inputs : List Input) {r : Record}
    (cited : r.citable = true) (h : r ∈ ledger s inputs) :
    r ∈ admitted (ledger s inputs) :=
  kept_by_admitted (fun said same => by subst same; simp [Record.citable] at cited) h

end HarnessRun
