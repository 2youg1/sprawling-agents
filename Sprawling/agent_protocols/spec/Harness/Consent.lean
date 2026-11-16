-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Nothing an ACP agent entry names is launched before the person consents to it.

Specifies `crates/agent_protocols/src/harness/entry.rs` (`Consented`) and the
one door that starts a child, `crates/agent_protocols/src/harness/process.rs`
(`HarnessProcess::start`), as `crates/agent_protocols/Spec.lean` section 8-19
and D16 describe them. The Rust code is the authority on how this holds; this
model is the authority on which property must hold.

The catalog, the detection and the paste parser only offer entries. An offer
starts nothing; consent turns one launch spec into something the process door
accepts, through one of two doors: the digest recorded when the person pressed
the consent card, which must equal the digest recomputed from the spec, or a
row the person wrote by hand. The model leaves the digest a parameter, because
what it needs from BLAKE3 is only that a recomputed digest is compared, not how
it is computed; the derived Rust check feeds the real digest.
-/

namespace AgentProtocols.Harness.Consent

/-- One launch spec: the program, its arguments and its environment. -/
structure Launch where
  program : String
  args : List String
  env : List (String × String)
  deriving DecidableEq, Repr

/-- The two ways a spec becomes consented (`Consented::given`,
`Consented::written_by_hand`). -/
inductive Door where
  | given (digest : Nat)
  | byHand
  deriving DecidableEq, Repr

/-- What happens to the roster: an offer is shown, a consent is asked for
through a door, a launch is asked for. -/
inductive Event where
  | offer (spec : Launch)
  | consent (spec : Launch) (door : Door)
  | launch (spec : Launch)
  deriving DecidableEq, Repr

/-- Whether a door admits a spec: the recorded digest must be the one
recomputed from the spec itself. -/
def admits (digest : Launch → Nat) (spec : Launch) : Door → Bool
  | .given recorded => digest spec == recorded
  | .byHand => true

/-- The specs consented so far, and the specs started, in order. -/
structure State where
  consented : List Launch
  started : List Launch
  deriving Repr

def State.fresh : State := ⟨[], []⟩

/-- One event. A launch of a spec nobody consented to starts nothing. -/
def step (digest : Launch → Nat) (s : State) : Event → State
  | .offer _ => s
  | .consent spec door =>
      if admits digest spec door then { s with consented := spec :: s.consented } else s
  | .launch spec =>
      if spec ∈ s.consented then { s with started := s.started ++ [spec] } else s

def run (digest : Launch → Nat) (s : State) (events : List Event) : State :=
  events.foldl (step digest) s

/-- Every consented spec was admitted by a door in some consent event of
`all`. -/
def Justified (digest : Launch → Nat) (all : List Event) (spec : Launch) : Prop :=
  ∃ door, Event.consent spec door ∈ all ∧ admits digest spec door = true

/-- What holds between events: every consented spec is justified by `all`,
and every started spec was consented. -/
def Good (digest : Launch → Nat) (all : List Event) (s : State) : Prop :=
  (∀ spec ∈ s.consented, Justified digest all spec) ∧ (∀ spec ∈ s.started, spec ∈ s.consented)

theorem step_keeps_good (digest : Launch → Nat) (all : List Event) (s : State) (e : Event)
    (seen : e ∈ all) (good : Good digest all s) : Good digest all (step digest s e) := by
  obtain ⟨justified, consented⟩ := good
  cases e with
  | offer _ => exact ⟨justified, consented⟩
  | consent c door =>
    by_cases ok : admits digest c door = true
    · simp only [step, ok, ite_true]
      refine ⟨fun x hx => ?_, fun x hx => List.mem_cons_of_mem _ (consented x hx)⟩
      rcases List.mem_cons.mp hx with rfl | m
      · exact ⟨door, seen, ok⟩
      · exact justified x m
    · simp only [step, ok]
      exact ⟨justified, consented⟩
  | launch l =>
    by_cases held : l ∈ s.consented
    · simp only [step, held, ite_true]
      refine ⟨justified, fun x hx => ?_⟩
      rcases List.mem_append.mp hx with m | m
      · exact consented x m
      · rw [List.mem_singleton.mp m]; exact held
    · simp only [step, held]
      exact ⟨justified, consented⟩

theorem run_keeps_good (digest : Launch → Nat) (all : List Event) :
    ∀ (events : List Event) (s : State), (∀ e ∈ events, e ∈ all) →
      Good digest all s → Good digest all (run digest s events) := by
  intro events
  induction events with
  | nil => intro s _ good; exact good
  | cons e rest ih =>
    intro s seen good
    exact ih (step digest s e) (fun x hx => seen x (List.mem_cons_of_mem _ hx))
      (step_keeps_good digest all s e (seen e List.mem_cons_self) good)

/-- The property: on every trace, a spec that was started had been consented
to through a door that admitted that very spec. -/
theorem nothing_is_launched_before_consent (digest : Launch → Nat) (events : List Event)
    (spec : Launch) (started : spec ∈ (run digest State.fresh events).started) :
    Justified digest events spec := by
  have good := run_keeps_good digest events events State.fresh (fun _ h => h)
    ⟨fun _ h => by simp [State.fresh] at h, fun _ h => by simp [State.fresh] at h⟩
  exact good.1 spec (good.2 spec started)

/-- The guard bites: a door that let a launch through without asking whether
the spec was consented starts a spec nobody consented to. -/
def stepUnguarded (digest : Launch → Nat) (s : State) : Event → State
  | .launch spec => { s with started := s.started ++ [spec] }
  | other => step digest s other

theorem withoutGuard_launches_an_offer (digest : Launch → Nat) (spec : Launch) :
    spec ∈ ([Event.offer spec, Event.launch spec].foldl (stepUnguarded digest) State.fresh).started
      ∧ ¬ Justified digest [Event.offer spec, Event.launch spec] spec := by
  refine ⟨by simp [stepUnguarded, step, State.fresh], ?_⟩
  rintro ⟨door, mem, _⟩
  simp at mem

end AgentProtocols.Harness.Consent
