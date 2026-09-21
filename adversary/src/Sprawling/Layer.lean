-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Ground
import Sprawling.Check

/-!
# The third world: a city whose configuration has been written through the wire.

`Sprawling.Model` remembers which buildings stand and what is halted;
`Sprawling.Provider` remembers one attached endpoint. Neither of them writes a
file a person also edits, and that is the whole of what this world is about.

What a run is governed by lives in `CONFIG.toml`, one file per layer, and a
command that changes it is a read-modify-write of a file the person owns. Two
readings therefore have to agree after any sequence of such writes: the bytes on
disk, and the answer the city folds for the page that shows them. A design in
which they could disagree has a setting that was saved and does not apply, or a
page that shows a value the next run will not read — and neither of those is
visible from inside the process that wrote them.

**Nothing here parses TOML.** The assertions are relations between two readings
the city itself produced: the answer states the figure of the last write, the
file states that figure and states no earlier one, and the layer above states
none of them. A second TOML reader in this directory would be a second authority
on the file's grammar, which `adversary-SPEC.md` section 1 forbids.

## What this stands in for

The invariant this world exists to hold is the one a person meets on the
settings page: after any sequence of writes, the file on disk and the answer
read back agree. The frame a person's own preferences will travel on does not
exist yet, so the invariant is asserted where the product already carries it —
`configure_building` writing a building's own layer, `building_view` folding it
back — and it moves to that frame when the frame arrives, because the property
is about the file rather than about the command that wrote it.
-/

namespace Sprawling

open Lean (Json)

/-- The building whose configuration this world writes.

One of the three names the whole directory uses, so a counterexample printed
here reads like every other one. -/
def configured : String := "acme"

/-- The layer a `configure_building` writes: the building's own file. -/
def buildingLayer : String := s!"{configured}/.sprawling/CONFIG.toml"

/-- The layer above it, which this world writes nothing into. -/
def cityLayer : String := ".sprawling/CONFIG.toml"

/-- The instruction budgets this world writes, and why they look like this.

Four digits each, all distinct, and no one of them a substring of another: the
file is compared by asking whether it states a figure, so two budgets where one
reads inside the other would make "the earlier write is gone" untestable.
Fixed rather than drawn from a range for the reason `adversary-SPEC.md`
section 14 gives for the cast of addresses — a counterexample is worth reading
only when its values are ones a person recognises on sight. -/
def budgets : List Nat := [7001, 8112, 9223, 4334, 5445, 6556, 3667]

/-- A non-empty sequence of writes.

Non-empty because the property is about what the last write left behind, and a
sequence with no last write states nothing for the file to agree with. -/
def writeSequence (size : Nat) : Gen (List Nat) := do
  let head ← Gen.elements 7001 budgets
  let rest ← Gen.listOf size (Gen.elements 7001 budgets)
  return head :: rest

/-- Shorter sequences that are still sequences.

`shrinkList` can reach the empty list, which this property has no meaning for,
so the empty candidate is dropped rather than reported as a smaller failure. -/
def shrinkSequence (written : List Nat) : List (List Nat) :=
  (shrinkList written).filter (fun candidate => !candidate.isEmpty)

/-- Whether a reading of a file states one figure.

Substring rather than a parse, and the budgets above are chosen so that this
cannot confuse two of them. -/
def states (text : String) (budget : Nat) : Bool :=
  (text.splitOn (toString budget)).length > 1

/-- The text of one file of the city, as the door reads it.

A layer nobody has written has no file, and the city says so by name rather
than by answering an empty one: that answer is read here as the statement that
this layer states nothing, which is what it means for a configuration file.

Any other shape throws. An answer read as an empty file would satisfy every
assertion below about what a file does not state, so a reader that guessed
here would turn a defect into a pass. -/
def Door.readDocument (door : Door) (ground : Ground) (path : String) : IO String := do
  match ← door.ask ground.port (.document path) with
  | .accepted frames =>
    match answerOf "document" frames with
    | some body =>
      match (body.getObjVal? "text" >>= Json.getStr?).toOption with
      | some text => return text
      | none => throw <| IO.userError s!"a document answer carried no text: {path}"
    | none =>
      if (answerOf "unavailable" frames).isSome then return "" else
        throw <| IO.userError s!"the city did not answer with a document: {path}"
  | .denied complaint =>
    throw <| IO.userError s!"a file of the city could not be read: {complaint}"
  | .quiet => throw <| IO.userError s!"the city said nothing about {path}"

/-- The instruction budget one building's own layer states, folded by the city.

`none` is a building whose own layer states no sandbox, which is a different
answer from one that states a budget of zero. -/
def Door.readBudget (door : Door) (ground : Ground) (addr : String) : IO (Option Nat) := do
  match ← door.ask ground.port (.buildingView addr) with
  | .accepted frames =>
    match answerOf "building" frames with
    | some body =>
      match (body.getObjVal? "sandbox").toOption with
      | some sandbox => return (sandbox.getObjVal? "fuel" >>= Json.getNat?).toOption
      | none => throw <| IO.userError "a building answer carried no sandbox field"
    | none => throw <| IO.userError s!"the city did not answer with a building: {addr}"
  | other => throw <| IO.userError s!"a building could not be read: {other}"

/-- Writes one budget into the building's own layer and waits for the answer.

A refusal here is the finding, so it is returned rather than thrown: the
sequence that produced it is what the property prints. -/
def Door.writeBudget (door : Door) (ground : Ground) (budget : Nat) (index : Nat) :
    IO (Option String) := do
  match ← door.ask ground.port (.configureBuilding configured budget (idemKey (400 + index))) with
  | .denied complaint => return some s!"writing a budget of {budget} was refused: {complaint}"
  | .quiet => return some s!"writing a budget of {budget} was answered with silence"
  | .accepted _ => return none

/-- After any sequence of writes, the file and the answer state the same figure.

Three readings, and each one answers a way the two homes could come apart:

* the answer states the last budget written, so a page cannot show what the
  next run will not read;
* the file states that budget and no earlier one, so a write replaces the value
  rather than adding a second statement of it;
* the layer above states none of them, so a building's own choice was written
  at the rung a person edits rather than resolved upward into the city's.
-/
def writtenReadsBack (door : Door) (written : List Nat) : IO Verdict :=
  withGround door fun ground => do
    match ← door.ask ground.port (.createBuilding configured .minimal (idemKey 399)) with
    | .accepted _ => pure ()
    | other => return .broke s!"a building nobody had raised was not raised: {other}"
    match ← writeAll ground written 0 with
    | some why => return .broke why
    | none =>
      let last := written.getLast?
      let folded ← door.readBudget ground configured
      let own ← door.readDocument ground buildingLayer
      let above ← door.readDocument ground cityLayer
      if last != folded then
        return .broke s!"the city folded {folded} where the last write stated {last}"
      else if !(last.all (states own)) then
        return .broke s!"the building's own layer does not state {last}"
      else
        match written.reverse.drop 1 |>.find? (fun earlier =>
            last != some earlier && states own earlier) with
        | some earlier =>
          return .broke s!"the building's own layer still states an earlier write of {earlier}"
        | none =>
          match written.find? (states above) with
          | some leaked =>
            return .broke s!"a building's own budget of {leaked} was written into the city's layer"
          | none => return .held
where
  writeAll (ground : Ground) : List Nat → Nat → IO (Option String)
    | [], _ => return none
    | budget :: rest, index => do
      match ← door.writeBudget ground budget index with
      | some why => return some why
      | none => writeAll ground rest (index + 1)

end Sprawling
