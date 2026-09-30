-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Starting read-only tools while the model is still generating.

Specifies the early handover in `crates/gateway/src/anthropic/stream.rs`
(`completed_call`) and the rule the runtime turn loop keeps when it acts on
a handed-over call: `crates/runtime/src/turn/speculation.rs` starts the reads
and caches them by position, and `crates/runtime/src/turn/wave.rs` records
them (gateway-SPEC.md section 8, runtime-SPEC.md section 8-3). The Rust code is the authority on how these
properties hold; this model is the authority on which properties must hold.

The decoder hands over each tool call the moment its `content_block_stop`
arrives. The runtime may start such a call before the answer settles, and
keeps what it returns in a cache keyed by the call's position in the answer.
Nothing is appended to the Ledger until the answer settles; then the calls are
recorded in the order the model emitted them, each taking its cached result
when there is one and running otherwise.

Two properties, one theorem group each:

* **a speculative result is not an event** - the Ledger an answer produces is
  the same with the cache as without it, for every way the answer can end; a
  truncated or cancelled answer appends no tool call at all, so its cache is
  discarded whole;
* **Ledger order is serial order** - with the cache filled only for the
  read-only calls that precede the answer's first writing call, the settled
  Ledger equals the one serial execution appends, record for record.

The second property is why speculation stops at the first writing call: a read
started early sees the world as it was before the answer's calls ran, and a
write earlier in the same answer changes that world
(`speculating_past_a_write_changes_the_ledger`).
-/

namespace Speculating

/-- Whether running a tool call can change the world. -/
inductive Kind where
  | read
  | write
  deriving Repr, DecidableEq

/-- A tool call the decoder handed over, identified by its call id. -/
structure Call where
  id : Nat
  kind : Kind
  deriving Repr, DecidableEq

/-- The state a tool call reads and writes, abstracted to a version. -/
abbrev World := Nat

/-- What running a call returns and what it leaves behind. A read-only call
leaves the world as it found it; that is what makes it safe to start early. -/
structure Tools where
  answer : Call → World → Nat
  effect : Call → World → World
  readsOnly : ∀ c w, c.kind = .read → effect c w = w

/-- A Ledger record about a tool call. -/
inductive Event where
  | toolCalled (id : Nat)
  | toolResult (id : Nat) (result : Nat)
  deriving Repr, DecidableEq

/-- How the model's answer ended. -/
inductive Ending where
  | settled
  | truncated
  | cancelled
  deriving Repr, DecidableEq

/-- Serial execution: each call in emission order, each seeing the world the
calls before it left. -/
def serial (t : Tools) : World → List Call → List Event
  | _, [] => []
  | w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (t.answer c w) :: serial t (t.effect c w) rest

/-- Settled execution with a cache: the call at each position takes the result
cached for that position when there is one, and runs otherwise. -/
def withCache (t : Tools) : List (Option Nat) → World → List Call → List Event
  | _, _, [] => []
  | cached :: more, w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (cached.getD (t.answer c w)) ::
      withCache t more (t.effect c w) rest
  | [], w, c :: rest =>
    .toolCalled c.id :: .toolResult c.id (t.answer c w) :: withCache t [] (t.effect c w) rest

def Call.isRead (c : Call) : Bool := c.kind == .read

/-- What speculation computes: the read-only calls before the first writing
call, each run against the world the answer started from. The order they
finish in does not matter, because the cache is keyed by position. -/
def speculated (t : Tools) (w : World) (calls : List Call) : List (Option Nat) :=
  (calls.takeWhile Call.isRead).map (some <| t.answer · w)

/-- The records an answer appends. A cut answer appends no tool call: its
failure is the model call's, recorded where every model failure is. -/
def ledger (t : Tools) (w : World) (ending : Ending) (calls : List Call)
    (cache : List (Option Nat)) : List Event :=
  match ending with
  | .settled => withCache t cache w calls
  | .truncated | .cancelled => []

/-! ## Ledger order is serial order -/

theorem no_cache_is_serial (t : Tools) (w : World) (calls : List Call) :
    withCache t [] w calls = serial t w calls := by
  induction calls generalizing w with
  | nil => rfl
  | cons c rest ih => simp [withCache, serial, ih]

theorem speculation_keeps_serial_order (t : Tools) (w : World) (calls : List Call) :
    withCache t (speculated t w calls) w calls = serial t w calls := by
  induction calls with
  | nil => rfl
  | cons c rest ih =>
    cases hk : c.kind with
    | read =>
      have hr : c.isRead = true := by simp [Call.isRead, hk]
      simp only [speculated, List.takeWhile_cons, hr, List.map_cons, ite_true] at ih ⊢
      simp [withCache, serial, t.readsOnly c w hk, ih]
    | write =>
      have hr : c.isRead = false := by simp [Call.isRead, hk]
      simp [speculated, hr, no_cache_is_serial]

/-! ## A speculative result is not an event -/

theorem speculation_is_not_an_event (t : Tools) (w : World) (ending : Ending)
    (calls : List Call) :
    ledger t w ending calls (speculated t w calls) = ledger t w ending calls [] := by
  cases ending with
  | settled => simp [ledger, speculation_keeps_serial_order, no_cache_is_serial]
  | truncated | cancelled => rfl

theorem a_cut_answer_discards_its_cache (t : Tools) (w : World) (ending : Ending)
    (calls : List Call) (cache : List (Option Nat)) (cut : ending ≠ .settled) :
    ledger t w ending calls cache = [] := by
  cases ending with
  | settled => contradiction
  | truncated | cancelled => rfl

/-! ## Why speculation stops at the first writing call -/

/-- A world where a write bumps the version and a read reports it. -/
def versioned : Tools where
  answer c w := match c.kind with
    | .read => w
    | .write => 0
  effect c w := match c.kind with
    | .read => w
    | .write => w + 1
  readsOnly c w h := by simp [h]

/-- Starting every read early, including one after a write, is the rejected
design: the read reports the world before the write. -/
def everyReadEarly (t : Tools) (w : World) (calls : List Call) : List (Option Nat) :=
  calls.map fun c => if c.isRead then some (t.answer c w) else none

theorem speculating_past_a_write_changes_the_ledger :
    let calls := [⟨1, .write⟩, ⟨2, .read⟩]
    withCache versioned (everyReadEarly versioned 0 calls) 0 calls ≠
      serial versioned 0 calls := by decide

end Speculating
