-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# A snapshot plus the tail after it is the whole fold.

`storage::snapshot` keeps what a fold held after some line of the Ledger, together
with the seq of that line and its chain hash. A start that finds the snapshot
checks that one line and folds only the lines after it. That start is only as
good as one property: resuming from the state a fold reached at a cut, over the
lines after the cut, reaches the state a fold over the whole ledger reaches.

**This module states the property for every fold and every cut.** The fold is an
opaque step function over opaque lines, so no view the product folds can drift
from it: `Views`, `Standing` and the chain state `storage::LineCheck` itself are
each one instance. The Rust side holds the same property by `proptest` over
random ledgers and random cut points (`storage::snapshot`'s tests), where the fold
is the chain check and a resumed walk must accept the same lines and end in the
same state as a walk from genesis.

**What the line hash buys, and what it does not.** A snapshot names the ledger it
was cut from only through the hash of its last line. When that hash matches the
line at the same seq on disk, and the digest is injective over the covered lines
(`Sprawling.Chain` owns that hypothesis), the snapshot's lines are a prefix of the
ledger on disk; `resumeIsWhole` is stated over that prefix fact, so it needs no
digest at all.
-/

namespace Sprawling.Snapshot

/-- What a fold holds after `lines`, starting from `init`. -/
def fold {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) : σ :=
  lines.foldl step init

/-- A snapshot cut at `k`: the state after the first `k` lines. -/
def cut {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) (k : Nat) : σ :=
  fold step init (lines.take k)

/-- Folding the lines after the cut from the snapshot's state is the whole fold,
for every cut, including `0` (no snapshot) and cuts past the end (a snapshot of
the whole ledger, with an empty tail). -/
theorem snapshotPlusTailIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (lines : List α) (k : Nat) :
    fold step (cut step init lines k) (lines.drop k) = fold step init lines := by
  unfold cut fold
  rw [← List.foldl_append, List.take_append_drop]

/-- A snapshot taken over `pre` resumes correctly on any ledger that `pre` is a
prefix of: the ledger grew after the snapshot was written, and the tail is what
it grew by. -/
theorem resumeIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (pre lines : List α) (h : pre <+: lines) :
    fold step (fold step init pre) (lines.drop pre.length) = fold step init lines := by
  obtain ⟨tail, rfl⟩ := h
  unfold fold
  rw [List.drop_left, List.foldl_append]

end Sprawling.Snapshot
