-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Frame

/-!
# What a ledger's chain certifies, and what it does not.

Every line of the Ledger carries a `prev`: a digest of the line before it. One
Rust function decides that value — `kernel::ledger::chain_hash`, which is
`B3Hash::digest` over the line's canonical bytes — and one Rust step consumes it,
`memory::jsonl::open`'s check that a line's `prev` equals the digest of the line
above it. Everything else that trusts the Ledger trusts those two.

**This module never computes a digest.** The hashing function stays an opaque
parameter, so no value here can drift from `chain_hash`: replacing blake3 with
another function tomorrow leaves every statement in this file unchanged. What the
module owns instead is the assumption those two lines make and never state — that
the digest function is injective — and the reach of the chain, which stops one
line short of the whole file. Both are facts a reader assumes in whichever
direction suits them, so both are proved here rather than described:

* The claims a ledger owes are a function of its covered lines. **Free**: it is
  congruence, and it needs nothing of the digest function. This is the direction
  a check uses when it compares one recorded digest with another recorded digest.
* Whether two ledgers wrote the same covered lines is decided by the claims they
  carry. **Bought**: this needs the digest to be injective, a statement about
  blake3 rather than about this product. That hypothesis stands in the theorem's
  own statement instead of in a comment, and
  `aCoveredLineHidesWithoutInjectivity` exhibits a digest function for which the
  same conclusion is false — so the hypothesis is not decoration.
* The **last** line is outside all of it. Nothing after it hashed it, so a
  chained ledger's head is whatever the disk says it is;
  `theLastLineIsNotCertified` exhibits one list of records consistent with two
  ledgers that differ at their last line. No injectivity hypothesis closes that,
  and the theorem assumes none. Two consequences follow, and both are about code
  rather than about arithmetic: a check that means to test detection must aim at
  a covered line, because damage to the head is the product's recovery path
  rather than its refusal path; and no reader may treat a Ledger's own chain as
  evidence about its head.

**Why Lean rather than one more Rust test.** A Rust test asserts about the ledger
it can build, and a `proptest` samples the ledgers a strategy can draw. These
statements are about every list of lines and every digest function, and two of
them are negative — they say a claim is *not* available — which no sample can
show. `cargo xtask proof` holds a few kernel propositions against real MIR; what
is at stake here is the reach of a rule.

**Why this does not become a second authority.** It restates no rule of the
product's: no code consults the predicate below, no digest value appears anywhere
in it, and its carriers are the ones the tree already has, read as the disk holds
them — the file's lines as text, and the `Record` the wire and the reader agree
on. Where a verdict is needed, a check asks the product (`Door.verify` runs the
product's own offline verification) rather than this module. Each statement names
the Rust line it corresponds to.
-/

namespace Sprawling

/-- The `prev` a ledger owes, oldest first: the genesis digest for the first
line, and the digest of the line before it for every later one.

This is the rule `memory::jsonl::open` verifies on every open, written as the
sequence a whole ledger owes rather than as a loop over one file. `lines.dropLast`
is every line but the head — the set that has a successor to be hashed into — so
a ledger with no line owes no claim and a ledger of one line owes exactly the
genesis digest.

Appending a line adds one entry at the end of this sequence and moves nothing
already there; `appendingALineMovesNoClaim` below is that fact as a theorem.

`hash` is `kernel::ledger::chain_hash` with the dependency it is built on left
open: a `String` in, a `String` out, and no value computed here. -/
def required (hash : String → String) (genesis : String) (lines : List String) : List String :=
  match lines with
  | [] => []
  | _ :: _ => genesis :: lines.dropLast.map hash

/-- A ledger is chained when the claims its records carry are the claims its
lines owe.

The two arguments are the two halves of one file as this machine holds it: the
lines as text, in order, and the records a reader parses out of them. They are
not independent — the equation below forces the two lists to the same length —
and a ledger whose records do not line up with its lines is not a chained ledger
but a parse that disagreed with the file.

The digest function and the genesis digest are parameters because neither is this
directory's fact: `chain_hash` lives in `crates/kernel/src/ledger.rs`, and its
value and `GENESIS_PREV` are read from the product. A copy of either here would
be the second home `adversary-SPEC.md` section 5 forbids. -/
def Chained (hash : String → String) (genesis : String) (lines : List String)
    (records : List Record) : Prop :=
  records.map Record.prev = required hash genesis lines

/-- A record carrying `prev` and nothing else this module reads.

The witnesses below vary the claim and the line's bytes, because those are what a
chain is about; every other field is the fixture's. -/
private def carrying (prev : String) : Record := { (default : Record) with prev }

/-- **The covered lines decide the claims, and this direction is free.**

Two readings of one ledger — the same number of lines, agreeing on every line but
the head — owe the same claims. `chain_hash` does not enter: *any* function has
this property, which is why a check may compare two recorded digests without
knowing how either was computed.

The line that relies on it compares a digest recorded earlier with a digest
recorded now: the four segment hashes `runtime::prefix` freezes once at run start
and every turn restates. The number of lines is part of the hypothesis because a
reading includes it — a file of no lines and a file of one line have the same
`dropLast`, and only the count tells them apart. -/
theorem theCoveredLinesDecideTheClaims (hash : String → String) (genesis : String)
    {lines lines' : List String} {records records' : List Record}
    (chained : Chained hash genesis lines records) (chained' : Chained hash genesis lines' records')
    (same : lines.dropLast = lines'.dropLast) (counted : lines.length = lines'.length) :
    records.map Record.prev = records'.map Record.prev := by
  have owed : required hash genesis lines = required hash genesis lines' := by
    cases lines with
    | nil =>
      cases lines' with
      | nil => rfl
      | cons head rest =>
        exact absurd counted (by simp)
    | cons head rest =>
      cases lines' with
      | nil =>
        exact absurd counted (by simp)
      | cons other tail => simp [required, same]
  unfold Chained at chained chained'
  rw [chained, chained', owed]

/-- **The claims decide the covered lines, and this direction is bought.**

`injective` is the whole price of concluding, from the equality of the claims two
ledgers carry, that they wrote the same covered lines. In the product this is the
step `runtime::replay::verify_lines` takes on every offline check — it compares a
line's `prev` with the digest of the line above it and reads a match as "the past
is what it says" — the same step `memory::jsonl::open::recover_tail` takes when a
city resumes, and the one `memory::bundle::files::head_of` takes when it insists
on one chain. `aCoveredLineHidesWithoutInjectivity` below is the counterexample
that shows the hypothesis cannot be dropped.

The head is outside the conclusion, and that is the rule rather than an artefact
of the proof: `lines.dropLast` is exactly the set a successor has hashed. -/
theorem theClaimsDecideTheCoveredLines (hash : String → String) (genesis : String)
    (injective : Function.Injective hash) {lines lines' : List String}
    (same : required hash genesis lines = required hash genesis lines') :
    lines.dropLast = lines'.dropLast := by
  cases lines with
  | nil =>
    cases lines' with
    | nil => rfl
    | cons other tail => exact absurd same (by simp [required])
  | cons head rest =>
    cases lines' with
    | nil => exact absurd same (by simp [required])
    | cons other tail =>
      have covered : (head :: rest).dropLast.map hash = (other :: tail).dropLast.map hash := by
        simpa only [required, List.map_cons, List.cons.injEq, true_and] using same
      exact (List.map_inj_right injective).mp covered

/-- A covered line cannot move without the claims moving with it.

The statement the disk's hostile actions are aimed at: change a byte in any line
but the last, and the ledger no longer carries the claims its own lines owe — so
the reader that compares the two refuses it. `injective` is carried for the same
reason as above.

`Ground.corrupt` flips a byte in the *oldest* line for a second reason this
theorem does not cover: that is the one position no city process can be racing.
What this theorem says is that the oldest line is not the only position worth
attacking — every line with a successor is covered — so a check that tests
detection at one position has tested one position. -/
theorem aCoveredLineCannotChangeUnnoticed (hash : String → String) (genesis : String)
    (injective : Function.Injective hash) {lines lines' : List String} {records : List Record}
    (chained : Chained hash genesis lines records) (moved : lines.dropLast ≠ lines'.dropLast) :
    records.map Record.prev ≠ required hash genesis lines' := by
  intro agrees
  unfold Chained at chained
  have same : required hash genesis lines = required hash genesis lines' := by
    rw [← chained, agrees]
  exact moved (theClaimsDecideTheCoveredLines hash genesis injective same)

/-- **The last line is outside the chain, and no digest function brings it in.**

One list of records, two ledgers, both chained, differing at their last line: for
two different byte strings a digest function of any strength may give whatever it
likes, because nothing after them ever hashed them. `hash` is the identity here,
so this holds under the strongest digest function there is rather than under a
weak one.

This is why the hostile actions split the way they do. `Ground.tear` removes a
tail and the product *recovers*, which is a promise it can keep because no claim
broke; `Ground.corrupt` must aim at a line a successor covers, or a check meaning
to test detection would be testing the recovery path. It is also why no reader
may treat the chain as evidence about its head: what the head says is what the
disk says. -/
theorem theLastLineIsNotCertified (genesis a b c : String) (different : b ≠ c) :
    ∃ lines lines' records,
      lines ≠ lines' ∧ Chained id genesis lines records ∧ Chained id genesis lines' records := by
  refine ⟨[a, b], [a, c], [carrying genesis, carrying a], ?_, ?_, ?_⟩
  · intro same
    injection same with _ rest
    injection rest with bytes _
    exact different bytes
  · simp [Chained, required, carrying, List.dropLast_eq_take]
  · simp [Chained, required, carrying, List.dropLast_eq_take]

/-- **Without injectivity a covered line hides as well as the head does.**

One list of records, two ledgers, both chained, differing at the line a successor
hashed — for a digest function that maps every line to the same string. This is
the case `theClaimsDecideTheCoveredLines` and `aCoveredLineCannotChangeUnnoticed`
exclude by hypothesis, and it is not a corner case: it is what "the assumption
does not hold" looks like. The strength of the tree's verification equals the
strength of that assumption and of nothing else.

Where the assumption is written down is the part that matters. It is not proved
here and cannot be: it is a statement about blake3. What the tree can do is say
which of its promises rest on it, which `adversary-SPEC.md` section 5 does — so
the next reader who needs a stronger guarantee knows what they are changing
rather than discovering a hole. -/
theorem aCoveredLineHidesWithoutInjectivity (genesis a z b : String) (different : a ≠ z) :
    ∃ lines lines' records,
      lines ≠ lines' ∧ Chained (fun _ => "") genesis lines records
        ∧ Chained (fun _ => "") genesis lines' records := by
  refine ⟨[a, b], [z, b], [carrying genesis, carrying ""], ?_, ?_, ?_⟩
  · intro same
    injection same with first _
    exact different first
  · simp [Chained, required, carrying, List.dropLast_eq_take]
  · simp [Chained, required, carrying, List.dropLast_eq_take]

/-- A ledger's head survives dropping the last line, in the only shape the
proofs below need: a list of two or more elements loses its last one and keeps
what a reader had already verified as its front.

Stated here because `List` does not carry it: `dropLast` on a single-element list
removes that element, so the step from three elements down is the first one that
walks from the left. -/
private theorem dropLastConsCons {α : Type} (first second : α) (rest : List α) :
    (first :: second :: rest).dropLast = first :: (second :: rest).dropLast := by
  simp [List.dropLast_eq_take]

/-- What a reader has verified of a ledger stays verified when the city writes
one more line.

`jsonl::append` computes one digest per line written and never re-reads what is
behind it; this is the same fact stated on the sequence of claims, where it
matters that the claims already owed do not move. It is the claims that are kept,
not the obligation of the new line — that one is `Chained`'s own definition,
since what a new record must carry is the next entry of `required`. -/
theorem appendingALineMovesNoClaim (hash : String → String) (genesis : String)
    (lines : List String) (line : String) :
    required hash genesis lines <+: required hash genesis (lines ++ [line]) := by
  cases lines with
  | nil => simp [required]
  | cons first rest =>
    have covered : (first :: rest).dropLast.map hash <+: (first :: rest).map hash := by
      revert first
      induction rest with
      | nil => intro first; simp
      | cons second more ih =>
        intro first
        rw [dropLastConsCons]
        simp only [List.map_cons]
        exact List.cons_prefix_cons.mpr ⟨rfl, ih second⟩
    simp only [required, List.dropLast_concat]
    exact List.cons_prefix_cons.mpr ⟨rfl, covered⟩

end Sprawling
