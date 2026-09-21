-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Layer

/-!
# The fourth world: the one layer that is the person's rather than a city's.

`Sprawling.Layer` asks whether a building's own `CONFIG.toml` and the answer
folded from it can be driven apart. This module asks the same question one rung
further out, where the file is not inside any city: what a person settles about
how they read their cities lives in `<home>/.sprawling/config.toml`, is written
by `Command::PutPreferences` and is read back whole by `Query::Preferences`.

**Two homes for one fact, and a machine boundary between them.** The file is
edited by hand as well as through the wire, and it is the file a new browser on
this machine is drawn from — so a value the city answers with and a value the
file states have to be the same value after any sequence of writes. A design in
which they could differ has a setting that was saved and is not applied, or a
page showing a figure the next process will not read.

**Nothing here parses TOML**, for the reason `Sprawling.Layer` gives: a second
reader of that grammar in this directory would be a second authority on it. The
assertions are relations between readings the product itself produced, plus the
one substring probe the values are chosen to make unambiguous.

**The city is asked as well, and must state nothing.** A preference is the
person's and travels with the machine; a city carried to another machine must
arrive without it. The city's own `CONFIG.toml` is therefore read at the end of
every sequence and must state none of what was written.
-/

namespace Sprawling

open Lean (Json)

/-- The font stack one figure is written as: the one spelling the writer and
every reader below share.

The figures are `Sprawling.Layer`'s, taken rather than chosen again. What makes
a value usable here is that no one of them reads inside another, and that
property is established once, beside the figures, together with the reasoning
that chose them. The words around the figure are what turn it into something a
person would plausibly type into a font box. -/
def stackOf (figure : Nat) : String :=
  s!"Adversary {figure} Sans"

/-- The sans-serif stack this person's preferences state, as the city folds
them.

`none` is a person who has stated no stack, which is a different answer from
one who stated the empty string.

A city that does not answer this query throws rather than reporting nothing:
`Answer::Unavailable` is a real answer meaning "this build does not evaluate
that view", and reading it as an absent preference would turn a view nobody
built into a property that passes. -/
def Door.readSansStack (door : Door) (ground : Ground) : IO (Option String) := do
  match ← door.ask ground.port .preferences with
  | .accepted frames =>
    match answerOf "preferences" frames with
    | some body =>
      match (body.getObjVal? "appearance" >>= (·.getObjVal? "sans_stack")
          >>= Json.getStr?).toOption with
      | some stated => return some stated
      | none => throw <| IO.userError "a preferences answer carried no appearance.sans_stack"
    | none =>
      match answerOf "unavailable" frames with
      | some _ =>
        throw <| IO.userError
          "this city answers Query::Preferences with Unavailable, so the person's layer \
           has no door yet"
      | none => throw <| IO.userError "the city did not answer with preferences"
  | other => throw <| IO.userError s!"the person's preferences could not be read: {other}"

/-- The person's layer as text, and the empty string for a layer nobody has
written yet.

Read off the disk rather than through the `document` query, because that query
answers for files of the city and this file is outside every city. Taking that
position is taking the one the design already grants: the file is in this
person's own directory, and anybody who can read it can write it. -/
def Ground.personText (ground : Ground) : IO String := do
  if ← ground.personLayer.pathExists then
    IO.FS.readFile ground.personLayer
  else
    return ""

/-- Writes one stack into the person's layer and waits for the answer.

A refusal is the finding, so it is returned rather than thrown: the sequence
that produced it is what the property prints.

**Silence is acceptance for this one command.** Every other command on this
wire is seen through the record it appends to the city's one history, and that
record is what the door reads back as frames. A preference is the person's and
no run can observe it, so nothing is appended and the city has nothing to
broadcast — a frame invented here would be the city stating, in its own
history, something that belongs to whoever is sitting at this machine. What
proves the write took effect is the read-back below, which is stronger than an
acknowledgement: it is the file and the answer, compared. -/
def Door.writeSansStack (door : Door) (ground : Ground) (figure : Nat) (index : Nat) :
    IO (Option String) := do
  match ← door.ask ground.port (.putAppearance (stackOf figure) (idemKey (500 + index))) with
  | .denied complaint => return some s!"writing the stack for {figure} was refused: {complaint}"
  | .quiet => return none
  | .accepted _ => return none

/-- After any sequence of `PutPreferences`, the file on disk and the answer
`Query::Preferences` gives are the same configuration.

Four readings, and each one answers a way the two homes could come apart:

* the answer states the last stack written, so a page cannot show what the next
  process will not read;
* the file states that stack, so an answer held in memory was actually saved;
* the file states no earlier stack, so a write replaces the value rather than
  adding a second statement of it;
* the city states none of them, so a preference stayed the person's and a city
  copied to another machine arrives without it.
-/
def preferencesReadBack (door : Door) (written : List Nat) : IO Verdict :=
  withGround door fun ground => do
    match ← writeAll ground written 0 with
    | some why => return .broke why
    | none =>
      let last := written.getLast?
      let folded ← door.readSansStack ground
      let onDisk ← ground.personText
      let cityOwn ← door.readDocument ground cityLayer
      if last.map stackOf != folded then
        return .broke
          s!"the city folded {folded} where the last write stated {last.map stackOf}"
      else if !(last.all fun figure => states onDisk (stackOf figure)) then
        return .broke s!"the person's layer does not state {last.map stackOf}"
      else
        match written.reverse.drop 1 |>.find? (fun earlier =>
            last != some earlier && states onDisk (stackOf earlier)) with
        | some earlier =>
          return .broke s!"the person's layer still states an earlier write of {stackOf earlier}"
        | none =>
          match written.find? (fun figure => states cityOwn (stackOf figure)) with
          | some leaked =>
            return .broke s!"a person's own preference {stackOf leaked} was written into the city"
          | none => return .held
where
  writeAll (ground : Ground) : List Nat → Nat → IO (Option String)
    | [], _ => return none
    | figure :: rest, index => do
      match ← door.writeSansStack ground figure index with
      | some why => return some why
      | none => writeAll ground rest (index + 1)

end Sprawling
