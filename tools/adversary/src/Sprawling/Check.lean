-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Drawing traces, shrinking them, and reporting what survived.

Three things this directory needs and the toolchain does not offer: drawing a
random value, making a failing one smaller, and running a named tree of checks
one at a time. They live here, together, because they are one job — a shrinker
that could not re-run a check would have nothing to test a candidate with.
`adversary-SPEC.md` section 13 records what writing them costs and what it buys.

**This module knows nothing about a city.** It draws values, shrinks them, runs
a named tree of checks, and reports. Everything about what a trace *means* is
`Sprawling.Model`'s. The split is enforced by the types below rather than by
discipline: `Gen` is a pure function of a seed, so nothing that generates a
trace can reach `IO` and consult the city it is about to attack.
-/

namespace Sprawling

/-- A draw from a pseudo-random source.

A function rather than a monad transformer over `IO`: a generator that could
perform an effect could consult the city it is generating a trace for, and the
whole value of a random trace is that it was chosen without looking. -/
structure Gen (α : Type) where
  draw : StdGen → α × StdGen

namespace Gen

instance : Monad Gen where
  pure value := ⟨fun seed => (value, seed)⟩
  bind generator continue' := ⟨fun seed =>
    let (value, next) := generator.draw seed
    (continue' value).draw next⟩

/-- A natural between `lo` and `hi`, both admitted. -/
def chooseNat (lo hi : Nat) : Gen Nat :=
  ⟨fun seed => randNat seed lo hi⟩

/-- One of the values offered, or the fallback when none are.

The fallback keeps this total. A partial `elements` would put a crash inside the
generator, and an adversary that crashes on its own bookkeeping reports its own
defects as the product's. -/
def elements (fallback : α) : List α → Gen α
  | [] => pure fallback
  | values => do
    let index ← chooseNat 0 (values.length - 1)
    return (values[index]?).getD fallback

/-- One of the generators offered, chosen in proportion to its weight. -/
def frequency (fallback : Gen α) (weighted : List (Nat × Gen α)) : Gen α :=
  let total := weighted.foldl (fun sum entry => sum + entry.fst) 0
  if total == 0 then fallback
  else do
    let point ← chooseNat 1 total
    pick point weighted
where
  pick (point : Nat) : List (Nat × Gen α) → Gen α
    | [] => fallback
    | (weight, generator) :: rest =>
      if point ≤ weight then generator else pick (point - weight) rest

/-- A list of at most `size` values. -/
def listOf (size : Nat) (each : Gen α) : Gen (List α) := do
  let length ← chooseNat 0 size
  many length
where
  many : Nat → Gen (List α)
    | 0 => pure []
    | n + 1 => do
      let head ← each
      return head :: (← many n)

end Gen

/-- Shorter lists that a failing list might still fail as.

Chunks first, halving: a trace of forty actions reaches four in a handful of
steps rather than in thirty-six, because removing one element at a time pays a
whole city per step. Single removals follow, so the last few steps can still be
taken one at a time.

Elements themselves are never made smaller. Every value a trace carries is
already one word of a fixed cast, and `adversary-SPEC.md` section 14 records why
the cast is fixed: a counterexample is worth reading only if its addresses are
names a person recognises. -/
private def removeAt (values : List α) (index count : Nat) : List α :=
  values.take index ++ values.drop (index + count)

/-- Every way to cut `size` neighbouring elements out, then the same for half
that many, down to one. -/
private def chunksOf (values : List α) : Nat → List (List α)
  | 0 => []
  | size + 1 =>
    let starts := List.range (values.length + 1 - min values.length (size + 1))
    starts.map (fun start => removeAt values start (size + 1)) ++ chunksOf values ((size + 1) / 2)
decreasing_by exact Nat.div_lt_self (Nat.succ_pos _) (by omega)

def shrinkList (values : List α) : List (List α) :=
  chunksOf values (values.length / 2) ++ singles
where
  singles :=
    if values.length ≤ 1 then []
    else (List.range values.length).map (fun index => removeAt values index 1)

/-- How a check ended, and what to print when it did not end well. -/
inductive Verdict where
  | held
  | broke (why : String)
deriving Inhabited

/-- What a property found, once it had shrunk as far as it could. -/
structure Finding where
  seed : Nat
  shrinks : Nat
  counterexample : String
  why : String

/-- Runs one property: draw, run, and on a failure shrink until nothing smaller
fails.

The shrink budget is bounded rather than exhaustive. Each candidate costs a
whole city — raised, served, driven and thrown away — so an unbounded search
would spend minutes improving a counterexample a person can already read. -/
def forAll (samples : Nat) (baseSeed : Nat) (generate : Gen α) (render : α → String)
    (shrink : α → List α) (attempt : α → IO Verdict) : IO (Option Finding) := do
  for sample in List.range samples do
    let seed := baseSeed + sample
    let (drawn, _) := generate.draw (mkStdGen seed)
    match ← attempt drawn with
    | .held => pure ()
    | .broke why =>
      let (smallest, smallestWhy, shrinks) ← reduce drawn why 0 24
      return some { seed, shrinks, counterexample := render smallest, why := smallestWhy }
  return none
where
  /-- The first smaller candidate that still fails replaces the current one, and
  the search starts again from there. -/
  reduce (current : α) (why : String) (shrinks : Nat) : Nat → IO (α × String × Nat)
    | 0 => return (current, why, shrinks)
    | budget + 1 => do
      match ← firstFailing (shrink current) with
      | none => return (current, why, shrinks)
      | some (smaller, smallerWhy) => reduce smaller smallerWhy (shrinks + 1) budget
  firstFailing : List α → IO (Option (α × String))
    | [] => return none
    | candidate :: rest => do
      match ← attempt candidate with
      | .broke why => return some (candidate, why)
      | .held => firstFailing rest

/-- A named tree of checks.

A group, a leaf, and one run at a time. Nothing here runs anything in parallel,
and `adversary-SPEC.md` section 16 records why: a served city owns a port, a
directory and a history, and two groups at once contend for all three. -/
inductive Tree where
  | leaf (name : String) (check : IO Unit)
  | group (name : String) (children : List Tree)

/-- Whether a path through the tree is one this run was asked for.

Two flags, `--select <text>` and `--reject <text>`, each matched against the
full path a check is reached by. They exist for the minute after a red: one
check reruns in seconds where the whole tree costs minutes, and the seed the
failure printed draws the same trace again. A pattern language would be a second
thing to learn for no answer it buys. -/
structure Selection where
  select : Option String := none
  reject : Option String := none

private def Selection.admits (selection : Selection) (path : String) : Bool :=
  let contains (needle : String) := (path.splitOn needle).length > 1
  (selection.select.all contains) && !(selection.reject.any contains)

/-- What a run of the tree came to. -/
structure Tally where
  passed : Nat := 0
  failed : Nat := 0
  skipped : Nat := 0

private def Tally.add (a b : Tally) : Tally :=
  { passed := a.passed + b.passed, failed := a.failed + b.failed,
    skipped := a.skipped + b.skipped }

/-- Runs the tree, one check at a time, and says what happened to each.

A failing check does not stop the run. A checker exists to report every promise
that broke, and stopping at the first would hide the rest behind it. -/
partial def Tree.run (selection : Selection) (path : String) : Tree → IO Tally
  | .leaf name check => do
    let full := if path.isEmpty then name else path ++ " / " ++ name
    if !selection.admits full then
      return { skipped := 1 }
    let started ← IO.monoMsNow
    try
      check
      let took := (← IO.monoMsNow) - started
      IO.println s!"  ok    {full} ({took} ms)"
      return { passed := 1 }
    catch error =>
      IO.println s!"  FAIL  {full}"
      IO.println s!"        {error}"
      return { failed := 1 }
  | .group name children => do
    let full := if path.isEmpty then name else path ++ " / " ++ name
    let mut tally : Tally := {}
    for child in children do
      tally := tally.add (← child.run selection full)
    return tally

/-- The assertion a check makes about a value it can name. -/
def ensure (holds : Bool) (why : String) : IO Unit :=
  if holds then pure () else throw (IO.userError why)

/-- The assertion a check makes about two values, which prints both when it
breaks.

Whole values rather than field by field: a difference a reader can see once is
worth more than five comparisons they have to assemble. -/
def ensureEq [BEq α] [ToString α] (expected actual : α) (why : String) : IO Unit :=
  ensure (expected == actual) s!"{why}\n        owed: {expected}\n        saw:  {actual}"

end Sprawling
