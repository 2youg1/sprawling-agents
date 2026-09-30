-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Going back, and the write domains of parallel runs.

Specifies `crates/storage/src/worktree/back.rs`, the tree a run opens when a
person goes back to a point in the city's history, together with the two
steps that share its write domain: a run writing a file in its own tree
(`crates/storage/src/worktree/trees.rs`) and a file restored from a point into
that tree (`crates/storage/src/worktree/back.rs`). The Rust code is the
authority on how these properties hold; this model is the authority on which
properties must hold (storage-SPEC.md 8-27).

The city's history is two things that already exist, and nothing else: the
one Ledger, which only grows, and the git objects of the city's repository,
which never change once written. A point is a commit. Going back to a point
opens a new run with a tree of its own whose branch starts at that commit;
the trunk the city stands on does not move, because another run may be
writing at that moment and the trunk is where its work lands. Undoing is not
deleting: it restores a file from an earlier point, and the Ledger records the
restore as one more record.

The model has four properties, with one group of theorems for each:

* **going back opens a tree at the point** - the new run's files are exactly
  the files of the commit it went back to;
* **going back refuses a tree that is live** - a name already held is not
  replaced, so no run's tree, the caller's own included, is ever overwritten
  by going back;
* **a step touches only its own run's tree** - going back, writing and
  restoring change the tree of the run that took the step and no other, and
  never the trunk; over any sequence of steps none of which is a run's own,
  that run's tree is the same at the end as at the start, so a write it made
  survives everything the other runs do;
* **the Ledger only grows** - every step appends exactly one record, and a
  restore (the undo) is one of them.

Merging a run's work into the trunk goes through the existing fast-forward
path (storage-SPEC.md 8-9) and is not modelled here.
-/

namespace GoingBack

/-- A commit in the city's repository: a point in history. -/
abbrev Commit := Nat
/-- The name of a run's tree (`WorktreeName`); one run, one name. -/
abbrev Name := Nat
/-- A file path inside a tree. -/
abbrev Path := Nat
/-- A file's contents; the git blob is what the Rust code stores. -/
abbrev Content := Nat

/-- One run's tree: the commit its branch started at and its files now. -/
structure Tree where
  base : Commit
  files : Path → Option Content

/-- What each step appends to the Ledger. -/
inductive Record where
  | forked (run : Name) (point : Commit)
  | wrote (run : Name) (path : Path)
  | restored (run : Name) (path : Path) (point : Commit)
  deriving Repr, DecidableEq

/-- The city: the commit the trunk stands on, the commits that exist, what
each commit holds (git objects are immutable, so this is a function), the
trees that are live, and the Ledger in append order. -/
structure City where
  trunk : Commit
  commits : List Commit
  history : Commit → Path → Option Content
  trees : Name → Option Tree
  ledger : List Record

/-- The trees with `n` holding `t` and every other name unchanged. -/
def put (trees : Name → Option Tree) (n : Name) (t : Tree) : Name → Option Tree :=
  fun m => if m = n then some t else trees m

/-- A tree with file `p` holding `v` and every other file unchanged. -/
def Tree.set (t : Tree) (p : Path) (v : Option Content) : Tree :=
  { t with files := fun q => if q = p then v else t.files q }

/-- Going back to `c` as the new run `n`: refused when `n` already holds a
tree or `c` is not a commit of the city. -/
def goBack (s : City) (n : Name) (c : Commit) : Option City :=
  if (s.trees n).isNone ∧ c ∈ s.commits then
    some { s with
      trees := put s.trees n ⟨c, s.history c⟩
      ledger := s.ledger ++ [.forked n c] }
  else none

/-- Run `n` writes `v` at `p` in its own tree; a run with no tree writes
nothing. -/
def write (s : City) (n : Name) (p : Path) (v : Content) : Option City :=
  match s.trees n with
  | none => none
  | some t => some { s with
      trees := put s.trees n (t.set p (some v))
      ledger := s.ledger ++ [.wrote n p] }

/-- Run `n` takes file `p` back from point `c` into its own tree. A file the
point does not hold is removed, which is what restoring it means. -/
def restore (s : City) (n : Name) (p : Path) (c : Commit) : Option City :=
  match s.trees n with
  | none => none
  | some t =>
    if c ∈ s.commits then
      some { s with
        trees := put s.trees n (t.set p (s.history c p))
        ledger := s.ledger ++ [.restored n p c] }
    else none

/-- One step any run may take. -/
inductive Step where
  | goBack (run : Name) (point : Commit)
  | write (run : Name) (path : Path) (content : Content)
  | restore (run : Name) (path : Path) (point : Commit)

/-- The run whose tree the step is about. -/
def Step.run : Step → Name
  | .goBack n _ | .write n _ _ | .restore n _ _ => n

def Step.apply : Step → City → Option City
  | .goBack n c, s => GoingBack.goBack s n c
  | .write n p v, s => GoingBack.write s n p v
  | .restore n p c, s => GoingBack.restore s n p c

/-- Steps taken in order; the first refusal stops the sequence. -/
def steps (s : City) : List Step → Option City
  | [] => some s
  | st :: rest =>
    match st.apply s with
    | none => none
    | some s' => steps s' rest

/-! ## Going back opens a tree at the point -/

theorem goBack_opens_at_the_point {s s' : City} {n : Name} {c : Commit}
    (h : goBack s n c = some s') :
    ∃ t, s'.trees n = some t ∧ t.base = c ∧ t.files = s.history c := by
  unfold goBack at h
  split at h
  · cases h
    exact ⟨⟨c, s.history c⟩, by simp [put], rfl, rfl⟩
  · contradiction

/-! ## Going back refuses a tree that is live -/

theorem goBack_refuses_a_live_tree {s : City} {n : Name} (c : Commit)
    (live : (s.trees n).isSome) : goBack s n c = none := by
  unfold goBack
  cases hn : s.trees n with
  | none => simp [hn] at live
  | some _ => simp

/-! ## A step touches only its own run's tree -/

theorem step_leaves_other_trees {st : Step} {s s' : City} {m : Name}
    (h : st.apply s = some s') (other : m ≠ st.run) :
    s'.trees m = s.trees m := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h
      simp [put, show m ≠ n from other]
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h
      simp [put, show m ≠ n from other]
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h
        simp [put, show m ≠ n from other]
      · contradiction

/-- No step moves the trunk or rewrites a commit; that is why going back
cannot disturb a run whose work is on its way to the trunk. -/
theorem step_keeps_trunk_and_history {st : Step} {s s' : City}
    (h : st.apply s = some s') :
    s'.trunk = s.trunk ∧ s'.commits = s.commits ∧ s'.history = s.history := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h; exact ⟨rfl, rfl, rfl⟩
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h; exact ⟨rfl, rfl, rfl⟩
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h; exact ⟨rfl, rfl, rfl⟩
      · contradiction

theorem others_never_touch_a_tree {s s' : City} {m : Name} :
    ∀ {trace : List Step}, steps s trace = some s' →
      (∀ st ∈ trace, st.run ≠ m) → s'.trees m = s.trees m := by
  intro trace
  induction trace generalizing s with
  | nil =>
    intro h _
    cases h
    rfl
  | cons st rest ih =>
    intro h others
    simp only [steps] at h
    split at h
    · contradiction
    · rename_i mid hmid
      have rest_same := ih h (fun st' mem => others st' (List.mem_cons_of_mem _ mem))
      have first_same := step_leaves_other_trees hmid
        (Ne.symm (others st List.mem_cons_self))
      rw [rest_same, first_same]

/-- A run's write survives any sequence of steps the other runs take,
going back included. -/
theorem a_write_survives_other_runs {s s₁ s₂ : City} {m : Name} {p : Path}
    {v : Content} {trace : List Step}
    (wrote : write s m p v = some s₁) (ran : steps s₁ trace = some s₂)
    (others : ∀ st ∈ trace, st.run ≠ m) :
    (s₂.trees m).bind (·.files p) = some v := by
  rw [others_never_touch_a_tree ran others]
  unfold write at wrote
  split at wrote
  · contradiction
  · cases wrote
    simp [put, Tree.set]

/-! ## The Ledger only grows -/

theorem step_appends_one_record {st : Step} {s s' : City}
    (h : st.apply s = some s') : ∃ r, s'.ledger = s.ledger ++ [r] := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h; exact ⟨_, rfl⟩
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h; exact ⟨_, rfl⟩
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h; exact ⟨_, rfl⟩
      · contradiction

theorem the_ledger_only_grows {s s' : City} :
    ∀ {trace : List Step}, steps s trace = some s' → s.ledger <+: s'.ledger := by
  intro trace
  induction trace generalizing s with
  | nil =>
    intro h
    cases h
    exact List.prefix_refl _
  | cons st rest ih =>
    intro h
    simp only [steps] at h
    split at h
    · contradiction
    · rename_i mid hmid
      obtain ⟨r, grown⟩ := step_appends_one_record hmid
      exact List.IsPrefix.trans ⟨[r], grown.symm⟩ (ih h)

/-- Undo is a restore, and a restore appends its record rather than removing
the one it undoes. -/
theorem undo_is_an_appended_restore {s s' : City} {n : Name} {p : Path}
    {c : Commit} (h : restore s n p c = some s') :
    s'.ledger = s.ledger ++ [.restored n p c] := by
  unfold restore at h
  split at h
  · contradiction
  · split at h
    · cases h; rfl
    · contradiction

end GoingBack
