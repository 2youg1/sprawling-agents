-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# A ledger's answer and what the reopened disk holds.

Specifies `crates/memory/src/jsonl/barrier.rs`, the state `Barrier` that
`append_all` consults before a wave and mends after its last sync. The Rust
code is the authority on how the ledger holds the property; this model is the
authority on which property must hold (storage-SPEC.md 8-1).

The disk is a list of cells: a whole record carrying its seq, or bytes a wave
left behind when its write died partway. Opening the ledger keeps the records
up to the first cell that is not one; that is tail recovery. A handle keeps a
position and the seqs it answered `Ok` for.

One property: **every seq a handle answered `Ok` for is held by the reopened
ledger**. It holds because a handle whose wave failed refuses every later
wave; `withoutBarrier` shows the trace that breaks it when the handle keeps
writing instead.

A wave is one record here. How many records a wave carries does not change
where the tear can fall relative to the records answered before it.
-/

namespace Durability

/-- What one position of a segment holds. -/
inductive Cell where
  | record (seq : Nat)
  | torn
  deriving DecidableEq, Repr

def Cell.isRecord : Cell → Bool
  | .record _ => true
  | .torn => false

/-- Tail recovery: the records up to the first cell that is not one. -/
def reopen (disk : List Cell) : List Cell :=
  disk.takeWhile Cell.isRecord

inductive Barrier where
  | whole
  | broken
  deriving DecidableEq, Repr

structure Ledger where
  disk : List Cell
  next : Nat
  barrier : Barrier
  claimed : List Nat

def Ledger.empty : Ledger := ⟨[], 0, .whole, []⟩

/-- What happens to one wave on the device. -/
inductive Outcome where
  /-- Written and synced: the wave answers `Ok`. -/
  | synced
  /-- The write died partway and left bytes that are not a record. -/
  | tore
  /-- The record reached the file but its sync failed. -/
  | unsynced

/-- One wave through a handle that keeps the barrier. -/
def Ledger.append (l : Ledger) : Outcome → Ledger
  | o => match l.barrier with
    | .broken => l
    | .whole => match o with
      | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
      | .tore => { l with disk := l.disk ++ [.torn], barrier := .broken }
      | .unsynced => { l with disk := l.disk ++ [.record l.next], barrier := .broken }

/-- The records `0 .. n` in order: what a whole disk holds. -/
def records (n : Nat) : List Cell :=
  (List.range n).map Cell.record

/-- What every reachable handle keeps: while the barrier stands the disk is
exactly the records below the position, and every answered seq is below it
and survives a reopen. -/
def Ledger.Sound (l : Ledger) : Prop :=
  (l.barrier = .whole → l.disk = records l.next) ∧
  ∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk

theorem takeWhile_records_append (n : Nat) (rest : List Cell) :
    (records n ++ rest).takeWhile Cell.isRecord
      = records n ++ rest.takeWhile Cell.isRecord := by
  induction n generalizing rest with
  | zero => simp [records]
  | succ k ih =>
    have split : records (k + 1) = records k ++ [Cell.record k] := by
      simp [records, List.range_succ]
    rw [split, List.append_assoc, ih]
    simp [List.takeWhile_cons, Cell.isRecord]

theorem reopen_records (n : Nat) : reopen (records n) = records n := by
  have h := takeWhile_records_append n []
  simpa [reopen] using h

theorem mem_records {s n : Nat} : Cell.record s ∈ records n ↔ s < n := by
  simp [records]

theorem empty_sound : Ledger.empty.Sound := by
  refine ⟨fun _ => rfl, ?_⟩
  simp [Ledger.empty]

/-- Every wave keeps a sound handle sound. -/
theorem append_sound (l : Ledger) (o : Outcome) (h : l.Sound) : (l.append o).Sound := by
  obtain ⟨shape, kept⟩ := h
  cases hb : l.barrier with
  | broken =>
    simp only [Ledger.append, hb]
    exact ⟨shape, kept⟩
  | whole =>
    have disk := shape hb
    have below : ∀ s ∈ l.claimed, s < l.next := by
      intro s hs
      have := kept s hs
      rw [disk, reopen_records] at this
      exact mem_records.mp this
    cases o with
    | synced =>
      have grown : l.disk ++ [Cell.record l.next] = records (l.next + 1) := by
        rw [disk]; simp [records, List.range_succ]
      refine ⟨fun _ => by simp [Ledger.append, hb, grown], ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [grown, reopen_records, mem_records]
      simp only [List.mem_append, List.mem_singleton] at hs
      rcases hs with hs | hs
      · exact Nat.lt_succ_of_lt (below s hs)
      · omega
    | tore =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [disk, reopen, takeWhile_records_append]
      simp [mem_records, below s hs]
    | unsynced =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [disk, reopen, takeWhile_records_append]
      simp [mem_records, below s hs]

/-- Any run of waves from an empty ledger leaves every answered seq on the
reopened disk. -/
theorem answered_survives_reopen (waves : List Outcome) :
    (waves.foldl Ledger.append Ledger.empty).Sound := by
  suffices ∀ l : Ledger, l.Sound → (waves.foldl Ledger.append l).Sound from
    this _ empty_sound
  induction waves with
  | nil => exact fun _ h => h
  | cons o rest ih => exact fun l h => ih _ (append_sound l o h)

/-- The handle without the barrier: a failed wave leaves the position where
it was and the next wave is written anyway. -/
def Ledger.appendAnyway (l : Ledger) : Outcome → Ledger
  | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
  | .tore => { l with disk := l.disk ++ [.torn] }
  | .unsynced => { l with disk := l.disk ++ [.record l.next] }

/-- The counterexample the barrier exists for: a torn wave, then a wave that
answers `Ok` and that tail recovery truncates away. -/
theorem withoutBarrier :
    let l := [Outcome.synced, .tore, .synced].foldl Ledger.appendAnyway Ledger.empty
    ¬ ∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk := by
  decide

end Durability
