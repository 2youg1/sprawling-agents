-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling

/-!
# Six groups, in the order that loses the least time when something breaks.

The renderer is checked first because it takes milliseconds and because a broken
deliverable makes every counterexample below it worthless. The door's own
contract comes next, then random traces, then the directed attack, then what the
history says about itself, then the disk's simplest lie.

With no binary to drive, this exits successfully and says so. A check that could
not run is not a check that failed, and treating it as one is how a second
toolchain would end up blocking Rust contributors.
-/

open Sprawling

/-- The seed every property starts from.

Fixed rather than drawn from the clock. A counterexample is only worth rendering
into a Rust test if the run that found it can be repeated, and a checker that
answers differently on two runs of the same tree cannot tell a fix from a lucky
draw. `SPRAWLING_SEED` moves it for a deliberate second opinion. -/
def baseSeed : IO Nat := do
  match ← IO.getEnv "SPRAWLING_SEED" with
  | some given => return given.toNat?.getD 20260912
  | none => return 20260912

/-- Runs an action against a city that is raised for it and thrown away after. -/
private def driving (door : Door) (act : Attempt α) : IO α :=
  withGround door fun ground => act.run { door, ground }

private def traceShown (trace : Trace) : String :=
  String.intercalate " ; " (trace.map toString)

/-! ## The deliverable -/

/-- The renderer is the authority for that Rust file, and the file is the proof
that what the renderer emits compiles and passes. Neither can move without the
other, and neither is a place where a rule could be restated. -/
private def remembered : Trace :=
  [ ⟨.nothing, .raise "acme" .minimal⟩
  , ⟨.nothing, .work "acme" "one"⟩
  , ⟨.nothing, .stop .city⟩
  , ⟨.nothing, .work "acme" "two"⟩ ]

/-- The trace this adversary keeps, and why it is this one.

With nothing attached, a dispatch stops at configuration and the caller is told
to go and attach a provider. Halt the city and the same dispatch must stop one
door earlier, at the gate, and say so — because the recovery line is the third
part of what a refusal promises, and "attach a provider" is advice that does not
lift a halt.

The last step is the one that would have gone wrong had those two guards been
written in the other order, and neither refusal is a value anybody computed:
both are promises the door makes to whoever is driving it. -/
private def deliverable : IO Unit := do
  let rendered := render "a_halted_city_names_the_halt_and_not_the_configuration" remembered
  let path : System.FilePath := ".." / "crates" / "sprawling" / "tests" / "from_adversary.rs"
  if (← IO.getEnv "SPRAWLING_ACCEPT") == some "1" then
    IO.FS.writeFile path rendered
  else
    ensure (← path.pathExists) s!"there is no {path}; rerun with SPRAWLING_ACCEPT=1"
    let onDisk ← IO.FS.readFile path
    ensure (rendered == onDisk)
      "the committed Rust test is not what this adversary renders; rerun with SPRAWLING_ACCEPT=1"

/-! ## The door's own contract -/

/-- `Sealed<T>` has no `Serialize`, and the wire's secret carrier is
uninhabited — so the frame is refused before a socket is even opened.

Asserted from outside because that is where it matters: the compile-time half of
this property is already proved by a trybuild case inside the repository, and
what nobody there can check is what an agent sees when it tries anyway. -/
private def sealed (door : Door) : IO Unit :=
  withGround door fun ground => do
    let answered ← door.askRaw ground.port
      "{\"command\":{\"put_secret\":{\"realm\":\"r\",\"name\":\"n\",\"value\":\"leak\"}}}"
    match answered with
    | .denied complaint =>
      ensureEq (Code.mk "E_WIRE_MISMATCH") complaint.code
        "a credential frame was refused for the wrong reason"
      ensure ((complaint.subject.splitOn "leak").length == 1)
        "the refusal repeated the value it was refusing to carry"
    | other => ensure false s!"a credential reached the city: {other}"

/-- What the door's own documentation promises: a refusal is distinguishable
from an acceptance, from outside, without reading the frames.

adversary-SPEC section 4 records the measurement that made this worth asserting:
the exit code means "no refusal arrived inside the quiet window", which is not
the same statement. -/
private def exitCodeMeansWhatItSays (door : Door) : IO Unit :=
  withGround door fun ground => do
    match ← door.ask ground.port (.createBuilding ".sprawling" .minimal (idemKey 0)) with
    | .denied _ => pure ()
    | other => ensure false s!"the reserved subtree was not defended: {other}"
    match ← door.ask ground.port .cityView with
    | .accepted _ => pure ()
    | other => ensure false s!"a plain query did not answer: {other}"

/-- A verb the wire spells and this city does not perform.

A refusal that names the verb and offers a way forward is the promise; going
silent, or answering with a code that means something else, is not. This is the
property that has to hold *before* anybody wires a control to it. -/
private def notBuiltStillAnswers (door : Door) : IO Unit :=
  withGround door fun ground => do
    let asked := Verb.takeover "00000000-0000-7000-8000-000000000000" (idemKey 1)
    match ← door.ask ground.port asked with
    | .denied complaint =>
      ensureEq (Code.mk "E_WIRE_MISMATCH") complaint.code
        "a verb that is not built answered with the wrong code"
      ensure (!complaint.recovery.isEmpty)
        "a verb that is not built refused without a way forward"
    | other => ensure false s!"a verb that is not built said: {other}"

/-! ## Properties over traces -/

/-- Reports a finding as a failure, with the trace that produced it. -/
private def report (found : Option Finding) : IO Unit :=
  match found with
  | none => pure ()
  | some finding =>
    throw <| IO.userError <|
      s!"seed {finding.seed}, {finding.shrinks} shrink(s)\n" ++
      s!"        trace: {finding.counterexample}\n" ++
      s!"        {finding.why}"

/-- Any trace at all, against a city raised for it and thrown away after.

Sample counts and trace lengths are both deliberately small. Each sample raises
a city, serves it, and spends one process per action, so the honest unit of cost
here is seconds per sample rather than samples per second; the shrinking is what
finds the defect, and it runs on whichever sample failed first.

The length bound is the load-bearing one. An action costs 0.3 s measured, and an
unbounded schedule would grow these traces past seventy actions, which buys
twenty-two seconds of the same three verbs repeated. What finds a defect here is
which verbs meet, not how many times. -/
private def traces (door : Door) : IO Unit := do
  report <| ← forAll 4 (← baseSeed) (arbitraryTrace 12) traceShown shrinkList fun trace =>
    driving door (runTrace initialState trace)

/-- Any prefix, one halt, any suffix, and work that must still be refused. -/
private def halting (door : Door) : IO Unit := do
  report <| ← forAll 3 (← baseSeed) (haltIsHonoured 6) traceShown shrinkList fun trace =>
    driving door (runTrace initialState trace)

/-- After any trace, the history the city wrote is one unbroken chain.

Neither half recomputes anything: the door's own offline verifier is asked
whether the chain holds. A city that forked its own history, or skipped a
number, would have a history nobody can replay — and replay is what
`sprawling resume` is built on. -/
private def chained (door : Door) : IO Unit := do
  report <| ← forAll 3 (← baseSeed) (arbitraryTrace 12) traceShown shrinkList fun trace =>
    driving door do
      match ← runTrace initialState trace with
      | .broke why => return .broke why
      | .held =>
        let kit ← read
        match ← kit.door.verify kit.ground.ledger with
        | .ok _ => return .held
        | .error why => return .broke s!"the history did not verify: {why}"

/-! ## What a refusal costs -/

/-- ARCHITECTURE section 5 makes the ordering load-bearing — every effect
becomes an event first — and `dispatch_in` opens by saying so in as many words:
a halted city that laid a job file down would leave a task in a room no run ever
opened.

So this dispatches to an address nobody raised. The city refuses. What it must
not leave behind is a building directory, a room inside it and a `JOB.md`, none
of which any record in the one history mentions. -/
private def nothingBehind (door : Door) : IO Unit :=
  withGround door fun ground => do
    let before ← ground.tree
    match ← door.ask ground.port (.dispatch "acme" "one" (idemKey 3)) with
    | .accepted _ => ensure false "a dispatch with no model attached was accepted"
    | .quiet => ensure false "a dispatch with no model attached said nothing"
    | .denied _ =>
      ensureEq before (← ground.tree) "a refused dispatch wrote into the city"

/-- The same ordering seen from the other side: through a query.

`city_view` answers by scanning directories, so a room a refused dispatch left
behind would be reported as a building. Nothing raised it, no `building_created`
record mentions it, and it has no rules of its own — but a person reading the
city would see it standing there. -/
private def listsOnlyRaised (door : Door) : IO Unit :=
  withGround door fun ground => do
    let _ ← door.ask ground.port (.dispatch "gamma" "one" (idemKey 4))
    match ← door.ask ground.port .cityView with
    | .accepted frames =>
      match answerOf "city" frames with
      | some body =>
        ensureEq (some ["hall"]) (cityBuildings body) "the city listed a building nobody raised"
      | none => ensure false "the city did not answer with a city"
    | other => ensure false s!"the city could not be read: {other}"

/-! ## The disk lies -/

/-- The simplest lie a disk can tell: one changed byte.

What the reader must never do is believe it. Nothing in the command asks for a
check, which is the point — verification is not an option a caller can forget. -/
private def tampering (door : Door) : IO Unit :=
  withGround door fun ground => do
    let _ ← door.ask ground.port (.createBuilding "acme" .minimal (idemKey 2))
    match ← door.verify ground.ledger with
    | .error why => ensure false s!"a clean history did not verify: {why}"
    | .ok _ =>
      ground.corrupt
      match ← door.verify ground.ledger with
      | .error complaint =>
        ensure ((complaint.splitOn "E_CAS_CORRUPT").length > 1)
          s!"a corrupted history was refused, but not as corruption: {complaint}"
      | .ok lines =>
        ensure false s!"a corrupted history verified as {lines} good line(s)"

/-- A tail torn off mid-record, which the product recovers from on purpose.

The one hostile action here whose answer is "this is fine". `memory::jsonl`
treats an unfinished last line as a write that did not land, so the history reads
back one record shorter and still verifies. Asserting the relation rather than a
length: what is owed is that the chain still holds and that it did not somehow
grow. -/
private def torn (door : Door) : IO Unit :=
  withGround door fun ground => do
    let _ ← door.ask ground.port (.createBuilding "acme" .minimal (idemKey 5))
    let whole ← door.verify ground.ledger
    ground.tear
    let recovered ← door.verify ground.ledger
    match whole, recovered with
    | .error why, _ => ensure false s!"a clean history did not verify: {why}"
    | .ok _, .error why => ensure false s!"a torn tail was refused rather than recovered: {why}"
    | .ok before, .ok after =>
      ensure (after ≤ before)
        s!"a torn history verified further than the whole one: {after} past {before}"

/-- One record written a second time, which the reader must not accept.

A repeated line carries a sequence number and a previous-hash that already
belong to the line above it, so a history that verified would be one two
different accounts of the past could explain — and `sprawling resume` builds what
it does on there being one. -/
private def doubled (door : Door) : IO Unit :=
  withGround door fun ground => do
    let _ ← door.ask ground.port (.createBuilding "acme" .minimal (idemKey 6))
    ground.duplicate
    match ← door.verify ground.ledger with
    | .error _ => pure ()
    | .ok verified =>
      ensure false s!"a history with a record written twice verified as far as {verified}"

/-- The same key on the same command twice, which must happen once.

Every state-changing command on this wire carries an `IdemKey`, and a key exists
to make a retry harmless: a client that sent a command, lost its connection and
sent it again must not have done it twice.

Observed through the door and nowhere else. The history's own verifier is asked
how far the chain runs before and after the second send, so what is asserted is
that the two readings agree — no sequence number is predicted and no rule is
recomputed here. -/
private def keyUsedTwice (door : Door) : IO Unit :=
  withGround door fun ground => do
    let key := idemKey 7
    let _ ← door.ask ground.port (.halt .city key)
    let once ← door.verify ground.ledger
    let _ ← door.ask ground.port (.halt .city key)
    let twice ← door.verify ground.ledger
    match once, twice with
    | .ok before, .ok after =>
      ensureEq before after
        "a command carrying a key the city had already seen was performed a second time"
    | _, _ =>
      ensure false s!"a history this check wrote did not verify: {once.toOption} then {twice.toOption}"

/-! ## The tree -/

private def properties (door : Door) : Tree :=
  .group "adversary"
    [ .leaf "the committed Rust test is what this adversary renders" deliverable
    , .group "the door"
        [ .leaf "a credential cannot be spelled onto the wire" (sealed door)
        , .leaf "the exit code says what the frames say" (exitCodeMeansWhatItSays door)
        , .leaf "a verb the city cannot perform still answers" (notBuiltStillAnswers door) ]
    , .leaf "a city admits exactly what its rules admit" (traces door)
    , .leaf "a halted city takes no work until it is released" (halting door)
    , .leaf "history reads back as one unbroken chain" (chained door)
    , .group "the disk lies"
        [ .leaf "a corrupted record is refused, not believed" (tampering door)
        , .leaf "a torn tail is recovered, not refused" (torn door)
        , .leaf "a record written twice is refused" (doubled door) ]
      -- Two symptoms of one cause. They keep their names because what they
      -- defend is the ordering that made them green — judgement before any
      -- effect — rather than any one line of it.
    , .group "a refusal costs nothing"
        [ .leaf "a refused dispatch leaves nothing on disk" (nothingBehind door)
        , .leaf "the city lists only what it raised" (listsOnlyRaised door) ]
      -- Alone, so that a red says which defect it is. Green the day something
      -- consults the key the wire makes every command carry.
    , .group "open finding: the idempotency key nothing reads"
        [ .leaf "a key the city has seen does not do the work again" (keyUsedTwice door) ] ]

private def selectionOf (args : List String) : Selection :=
  { select := valueAfter "--select" args, reject := valueAfter "--reject" args }
where
  valueAfter (flag : String) : List String → Option String
    | found :: value :: rest => if found == flag then some value else valueAfter flag (value :: rest)
    | _ => none

def main (args : List String) : IO UInt32 := do
  match ← discover with
  | none =>
    IO.println "skipped: no sprawling binary to drive. Run `just adversary`, or set SPRAWLING_BIN."
    return 0
  | some door =>
    let tally ← (properties door).run (selectionOf args) ""
    IO.println s!"{tally.passed} passed, {tally.failed} failed, {tally.skipped} skipped"
    return (if tally.failed == 0 then 0 else 1)
