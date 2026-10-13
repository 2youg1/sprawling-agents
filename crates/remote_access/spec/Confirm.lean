-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Console confirmation: a page asks, the console shows a code, the User types it.

Specifies `remote_access::confirm` (crates/remote_access/src/confirm.rs,
crates/remote_access/Spec.lean §8-13), and the two wire commands that
reach it (crates/wire/spec/Command/Kind.lean, the rows for the remote door).
The model is the authority on which properties must hold; the Rust module
is the authority on how they hold (remote_access D4).

Two verbs of the remote door widen what reaches the city or cannot be taken
back: opening the door and replacing the city key. A page on the city's
loopback port can ask for either, but any local client can send the same
frame, including a resident's browser tool that drives the page. So a request
does nothing by itself: the city draws a secret, prints it at the city's
console beside the verb it would do, and answers the page that a code is
pending, without the secret. Only a confirmation that carries that secret,
directly after the request and before it expires, does the verb.

The machine holds at most one pending request. A new request replaces the
earlier one, and every confirmation, right or wrong, ends the pending one, so
a wrong guess costs the guesser the request.

The model has three properties:

* **a verb is done only in answer to the request just before it** - the
  confirmation carries the secret that request showed, for the same verb,
  before the request expired;
* **an answer the console never showed does nothing** - a client that never
  read the console, whatever it sends, never gets a verb done;
* **an answer in time does the verb** - the User who reads the code and types
  it back gets the verb.

Those three hold for the answers the machine judges. Nothing above limits how
many answers a local client may send: it can loop request and wrong answer,
one 40-bit guess per round, as fast as the listener carries frames. So the
machine the Rust module implements is the one above behind a guard
(remote_access D25). The guard judges an answer only when the last judged
answer is at least `spacing` behind it, and once `limit` judged answers have
missed since the last one that did its verb, it refuses every request and
every answer. Two more properties hold for the guarded machine:

* **a guesser gets at most `limit` judged answers** - in any trace in which no
  verb is done, at most `limit` answers are judged and missed, so a client
  that never read the console wins with chance at most `limit` in 2^40;
* **the guard lets the User through** - from a guard that is not locked and
  whose last judged answer is far enough behind, a request answered in time
  still does the verb.
-/

namespace RemoteConfirm

/-- A point in time, in the unit the caller samples. -/
abbrev Time := Nat
/-- A confirmation code, by its secret. -/
abbrev Secret := Nat

/-- The verbs that wait for the console. Closing the door is not one: it only
takes access away (remote_access D5). -/
inductive Guarded where
  | openDoor
  | replaceKey
  deriving DecidableEq, Repr

/-- The one request waiting for its confirmation. -/
structure Pending where
  verb : Guarded
  secret : Secret
  expires : Time
  deriving DecidableEq, Repr

/-- What a local client sends: a request (the city draws `secret`), or a
confirmation carrying an answer. -/
inductive Event where
  | request (verb : Guarded) (secret : Secret) (now : Time)
  | confirm (answer : Secret) (now : Time)
  deriving DecidableEq, Repr

/-- What one event makes the city do. `shown` is printed at the console only;
the page is told that a code is pending, and never the secret. -/
inductive Outcome where
  | shown (secret : Secret)
  | done (verb : Guarded)
  | refused
  deriving DecidableEq, Repr

/-- One event, given how long a code lives. -/
def step (ttl : Time) (s : Option Pending) (e : Event) : Option Pending × Outcome :=
  match e, s with
  | .request v a now, _ => (some ⟨v, a, now + ttl⟩, .shown a)
  | .confirm a now, some p =>
      if a = p.secret ∧ now < p.expires then (none, .done p.verb) else (none, .refused)
  | .confirm _ _, none => (none, .refused)

/-- The outcomes of a trace, one per event, from state `s`. -/
def run (ttl : Time) : Option Pending → List Event → List Outcome
  | _, [] => []
  | s, e :: es => (step ttl s e).2 :: run ttl (step ttl s e).1 es

theorem step_done {ttl : Time} {s : Option Pending} {e : Event} {v : Guarded}
    (h : (step ttl s e).2 = .done v) :
    ∃ a now p, e = .confirm a now ∧ s = some p ∧ p.verb = v ∧ a = p.secret ∧
      now < p.expires := by
  cases e with
  | request v' a now => simp [step] at h
  | confirm a now =>
    cases s with
    | none => simp [step] at h
    | some p =>
      by_cases hc : a = p.secret ∧ now < p.expires
      · simp only [step, ite_eq_left hc] at h
        exact ⟨a, now, p, rfl, rfl, Outcome.done.inj h, hc.1, hc.2⟩
      · simp only [step, ite_eq_right hc] at h
        cases h

theorem step_pending {ttl : Time} {s : Option Pending} {e : Event} {p : Pending}
    (h : (step ttl s e).1 = some p) :
    ∃ t, e = .request p.verb p.secret t ∧ p.expires = t + ttl := by
  cases e with
  | request v a now =>
    simp only [step, Option.some.injEq] at h
    subst h
    exact ⟨now, rfl, rfl⟩
  | confirm a now =>
    cases s with
    | none => simp [step] at h
    | some q =>
      by_cases hc : a = q.secret ∧ now < q.expires
      · simp [step, ite_eq_left hc] at h
      · simp [step, ite_eq_right hc] at h

/-- The first event of a trace from rest never does a verb. -/
theorem nothing_done_first (ttl : Time) (es : List Event) (v : Guarded) :
    (run ttl none es)[0]? ≠ some (.done v) := by
  intro h
  cases es with
  | nil => simp [run] at h
  | cons e es =>
    simp only [run, List.getElem?_cons_zero, Option.some.injEq] at h
    obtain ⟨_, _, _, _, hs, _⟩ := step_done h
    cases hs

/-- A verb is done only in answer to the request just before it: same verb,
the secret that request showed, before it expired. -/
theorem done_answers_the_request_before_it (ttl : Time) :
    ∀ (es : List Event) (s : Option Pending) (i : Nat) (v : Guarded),
      (run ttl s es)[i + 1]? = some (.done v) →
      ∃ a t now, es[i]? = some (.request v a t) ∧ es[i + 1]? = some (.confirm a now) ∧
        now < t + ttl
  | [], _, _, _, h => by simp [run] at h
  | e :: es, s, 0, v, h => by
    cases es with
    | nil => simp [run] at h
    | cons e2 es2 =>
      simp only [run, List.getElem?_cons_succ, List.getElem?_cons_zero,
        Option.some.injEq] at h
      obtain ⟨a, now, p, he2, hs, hv, ha, hlt⟩ := step_done h
      obtain ⟨t, he, hexp⟩ := step_pending hs
      subst hv
      subst ha
      exact ⟨p.secret, t, now, by simp [he], by simp [he2], hexp ▸ hlt⟩
  | e :: es, s, k + 1, v, h => by
    simp only [run, List.getElem?_cons_succ] at h
    obtain ⟨a, t, now, h1, h2, h3⟩ := done_answers_the_request_before_it ttl es _ k v h
    exact ⟨a, t, now, by simpa using h1, by simpa using h2, h3⟩

/-- An answer the console never showed does nothing: if no confirmation in the
trace carries a secret some request in it drew, no verb is done. -/
theorem an_unshown_answer_does_nothing (ttl : Time) (es : List Event)
    (unseen : ∀ a now v t, Event.confirm a now ∈ es → Event.request v a t ∉ es)
    (v : Guarded) : Outcome.done v ∉ run ttl none es := by
  intro hmem
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.mp hmem
  cases i with
  | zero => exact nothing_done_first ttl es v hi
  | succ k =>
    obtain ⟨a, t, now, h1, h2, _⟩ := done_answers_the_request_before_it ttl es none k v hi
    exact unseen a now v t (List.mem_iff_getElem?.mpr ⟨_, h2⟩)
      (List.mem_iff_getElem?.mpr ⟨_, h1⟩)

/-- An answer in time does the verb, whatever was pending before. -/
theorem an_answer_in_time_does_the_verb (ttl : Time) (s : Option Pending)
    (v : Guarded) (a t now : Nat) (h : now < t + ttl) :
    run ttl s [.request v a t, .confirm a now] = [.shown a, .done v] := by
  simp [run, step, h]

/-! ## The guard: answers are spaced, and misses are counted -/

/-- What the guard keeps between answers: when the last judged answer came,
and how many judged answers missed since the last one that did its verb. -/
structure Guard where
  last : Option Time
  misses : Nat
  deriving DecidableEq, Repr

/-- The guard a city starts with. -/
def Guard.fresh : Guard := ⟨none, 0⟩

/-- Whether an answer at `now` is far enough behind the last judged one. A
clock set back counts as spaced: the miss count bounds a guesser, and a
corrected wall clock must not stall the User. -/
def Guard.spaced (spacing : Time) (g : Guard) (now : Time) : Bool :=
  match g.last with
  | none => true
  | some t => t + spacing ≤ now || now < t

/-- What one event comes to behind the guard. `missed` is a judged answer that
did no verb; `tooSoon` and `lockedOut` are answers the guard did not judge. -/
inductive Said where
  | shown (secret : Secret)
  | done (verb : Guarded)
  | missed
  | tooSoon
  | lockedOut
  deriving DecidableEq, Repr

/-- A judged answer: the machine above decides, the guard counts. -/
def judge (ttl : Time) (s : Option Pending) (g : Guard) (a now : Nat) :
    (Option Pending × Guard) × Said :=
  match step ttl s (.confirm a now) with
  | (s', .done v) => ((s', ⟨some now, 0⟩), .done v)
  | (s', _) => ((s', ⟨some now, g.misses + 1⟩), .missed)

/-- One event behind the guard. A locked guard refuses everything, requests
included, so the console prints no code nobody may answer; an answer too soon
after the last judged one is refused and leaves the pending request as it was. -/
def guarded (ttl spacing limit : Time) : Option Pending × Guard → Event →
    (Option Pending × Guard) × Said
  | (s, g), e =>
    if limit ≤ g.misses then ((s, g), .lockedOut) else
    match e with
    | .request v a now => ((some ⟨v, a, now + ttl⟩, g), .shown a)
    | .confirm a now =>
      if g.spaced spacing now then judge ttl s g a now else ((s, g), .tooSoon)

/-- The outcomes of a trace behind the guard. -/
def runGuarded (ttl spacing limit : Time) : Option Pending × Guard → List Event → List Said
  | _, [] => []
  | s, e :: es =>
    (guarded ttl spacing limit s e).2 :: runGuarded ttl spacing limit (guarded ttl spacing limit s e).1 es

/-- A judged answer either does the verb or misses. -/
theorem judge_done_or_missed (ttl : Time) (s : Option Pending) (g : Guard) (a now : Nat) :
    (∃ v s', judge ttl s g a now = ((s', ⟨some now, 0⟩), .done v)) ∨
      (∃ s', judge ttl s g a now = ((s', ⟨some now, g.misses + 1⟩), .missed)) := by
  unfold judge
  cases s with
  | none => exact Or.inr ⟨none, by simp [step]⟩
  | some p =>
    by_cases hc : a = p.secret ∧ now < p.expires
    · exact Or.inl ⟨p.verb, none, by simp [step, hc]⟩
    · exact Or.inr ⟨none, by simp [step, hc]⟩

/-- Without a verb done, the misses a trace adds stop where the guard locks. -/
theorem misses_stop_at_the_limit (ttl spacing limit : Time) :
    ∀ (es : List Event) (s : Option Pending) (g : Guard),
      (∀ v, Said.done v ∉ runGuarded ttl spacing limit (s, g) es) →
      (runGuarded ttl spacing limit (s, g) es).count .missed + g.misses ≤ max limit g.misses
  | [], _, g, _ => by simpa [runGuarded] using Nat.le_max_right limit g.misses
  | e :: es, s, g, none_done => by
    have rest_none : ∀ v, Said.done v ∉
        runGuarded ttl spacing limit (guarded ttl spacing limit (s, g) e).1 es := by
      intro v hv
      exact none_done v (by simp only [runGuarded]; exact List.mem_cons_of_mem _ hv)
    by_cases locked : limit ≤ g.misses
    · have step_is : guarded ttl spacing limit (s, g) e = ((s, g), .lockedOut) := by
        simp [guarded, locked]
      have ih := misses_stop_at_the_limit ttl spacing limit es s g (by simpa [step_is] using rest_none)
      simp only [runGuarded, step_is, List.count_cons]
      simpa using ih
    · cases e with
      | request v a now =>
        have step_is : guarded ttl spacing limit (s, g) (.request v a now) =
            ((some ⟨v, a, now + ttl⟩, g), .shown a) := by
          simp [guarded, locked]
        have ih := misses_stop_at_the_limit ttl spacing limit es _ g
          (by simpa [step_is] using rest_none)
        simp only [runGuarded, step_is, List.count_cons]
        simpa using ih
      | confirm a now =>
        by_cases spaced : g.spaced spacing now = true
        · rcases judge_done_or_missed ttl s g a now with ⟨v, s', hj⟩ | ⟨s', hj⟩
          · exfalso
            apply none_done v
            simp [runGuarded, guarded, locked, spaced, hj]
          · have step_is : guarded ttl spacing limit (s, g) (.confirm a now) =
                ((s', ⟨some now, g.misses + 1⟩), .missed) := by
              simp [guarded, locked, spaced, hj]
            have ih := misses_stop_at_the_limit ttl spacing limit es s' ⟨some now, g.misses + 1⟩
              (by simpa [step_is] using rest_none)
            have below : g.misses < limit := Nat.lt_of_not_le locked
            dsimp only at ih
            rw [Nat.max_eq_left below] at ih
            simp only [runGuarded, step_is, List.count_cons, Nat.max_eq_left (Nat.le_of_lt below)]
            simp
            omega
        · have step_is : guarded ttl spacing limit (s, g) (.confirm a now) = ((s, g), .tooSoon) := by
            simp [guarded, locked, spaced]
          have ih := misses_stop_at_the_limit ttl spacing limit es s g
            (by simpa [step_is] using rest_none)
          simp only [runGuarded, step_is, List.count_cons]
          simpa using ih

/-- A guesser gets at most `limit` judged answers: from the guard a city starts
with, a trace that does no verb misses at most `limit` times. -/
theorem a_guesser_gets_at_most_limit_answers (ttl spacing limit : Time) (es : List Event)
    (s : Option Pending) (none_done : ∀ v, Said.done v ∉ runGuarded ttl spacing limit (s, Guard.fresh) es) :
    (runGuarded ttl spacing limit (s, Guard.fresh) es).count .missed ≤ limit := by
  have bound := misses_stop_at_the_limit ttl spacing limit es s Guard.fresh none_done
  simpa [Guard.fresh] using bound

/-- The guard lets the User through: unlocked and spaced, a request answered in
time does the verb. -/
theorem the_guard_lets_the_user_through (ttl spacing limit : Time) (s : Option Pending)
    (g : Guard) (v : Guarded) (a t now : Nat) (unlocked : g.misses < limit)
    (spaced : g.spaced spacing now = true) (in_time : now < t + ttl) :
    runGuarded ttl spacing limit (s, g) [.request v a t, .confirm a now] = [.shown a, .done v] := by
  have open_ : ¬ limit ≤ g.misses := Nat.not_le.mpr unlocked
  simp [runGuarded, guarded, open_, spaced, judge, step, in_time]

end RemoteConfirm
