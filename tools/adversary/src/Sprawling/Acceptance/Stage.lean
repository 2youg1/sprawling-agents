-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Layer

/-!
# The directory a stranger raises a city in, and what they edit by hand.

The acceptance world is a person with a fresh folder, the binary out of a
release archive, and a text editor. Everything that person does through the city
goes through `Sprawling.Door`; what they do with the editor is written here, as
the files say a person writes it: naming the archive's `skills/` as a shelf and
admitting those skills in a building's reading room (`docs/getting-started.md`,
"Tools, skills and MCP"), asking a building to review its work, and writing the
first row of its plan (`crates/city/templates/RULES.toml`,
`crates/city/templates/Roadmap.md`).

Unlike `withGround`, the directory outlives one served process: the walk serves
it, kills it in the middle of a run, and serves it again.
-/

namespace Sprawling.Acceptance

open Sprawling

/-- One folder: the city, and the home the served city is told is the person's. -/
structure Stage where
  root : System.FilePath
  city : System.FilePath
  home : System.FilePath

/-- Raises a city in a fresh folder, with a home beside it. -/
def Stage.raise (door : Door) : IO Stage := do
  let root ← IO.FS.createTempDir
  let stage : Stage := { root, city := root / "city", home := root / "home" }
  IO.FS.createDirAll stage.home
  door.raise stage.city
  return stage

/-- Throws the folder away. -/
def Stage.discard (stage : Stage) : IO Unit :=
  try IO.FS.removeDirAll stage.root catch _ => pure ()

/-- Serves this folder's city for as long as `act` runs, and kills it after. -/
def Stage.serving (stage : Stage) (door : Door) (act : Ground → IO α) : IO α :=
  servingAt door stage.city stage.home act

/-- The skills a release ships: every directory on the shelf holding a
`SKILL.md`, by its directory name, sorted.

Read off the archive rather than listed, so a skill added to `skills/` is
walked by the next run without a line changing here. -/
def shipped (shelf : System.FilePath) : IO (List String) := do
  let entries ← shelf.readDir
  let named ← entries.toList.filterMapM fun entry => do
    if ← (entry.path / "SKILL.md").pathExists then
      return some entry.fileName
    else
      return none
  return (named.toArray.qsort (· < ·)).toList

/-- Names the archive's shelf in the city's own configuration.

Appended as a table of its own to the layer `Sprawling.Layer` already names,
which is the form the guide gives. A file that already names shelves is refused:
appending a second `[skills]` table would make the file one TOML cannot read,
and the walk would then report the city for a file this module wrote. -/
def mountShelf (city shelf : System.FilePath) : IO Unit := do
  if !shelf.isAbsolute then
    throw <| IO.userError s!"the shelf has to be named by an absolute path: {shelf}"
  let layer := city / cityLayer
  let written ← if ← layer.pathExists then IO.FS.readFile layer else pure ""
  if (written.splitOn "[skills]").length > 1 then
    throw <| IO.userError s!"{layer} already names shelves, so this walk cannot add its own"
  IO.FS.writeFile layer (written ++ s!"\n[skills]\nshelves = ['{shelf}']\n")

/-- Where a building's rules live, relative to the city. -/
def rulesLayer (addr : String) : String := s!"{addr}/.sprawling/RULES.toml"

/-- The empty reading room a building is raised with. -/
private def emptyRoom : String := "reading_room = []"

/-- Admits the skills named into a building's reading room.

Replaces the empty list the template writes, as a person editing the file
would. A file that does not hold that line is refused rather than appended to:
the template has changed, and a second `reading_room` key would be a file the
city refuses for a reason this module caused. -/
def admit (city : System.FilePath) (addr : String) (skills : List String) : IO Unit := do
  let rules := city / rulesLayer addr
  let written ← IO.FS.readFile rules
  if (written.splitOn emptyRoom).length != 2 then
    throw <| IO.userError s!"{rules} does not hold the line `{emptyRoom}` once"
  let named := ", ".intercalate (skills.map fun skill => s!"\"{skill}\"")
  IO.FS.writeFile rules (written.replace emptyRoom s!"reading_room = [{named}]")

/-- The line a building is raised with that says its work lands unreviewed. -/
private def unreviewed : String := "review = false"

/-- Asks a building to review its work before it lands.

Replaces the one line the template writes, as a person editing the file would,
and refuses a file that does not hold it exactly once, for the reason `admit`
gives. -/
def askForReview (city : System.FilePath) (addr : String) : IO Unit := do
  let rules := city / rulesLayer addr
  let written ← IO.FS.readFile rules
  if (written.splitOn unreviewed).length != 2 then
    throw <| IO.userError s!"{rules} does not hold the line `{unreviewed}` once"
  IO.FS.writeFile rules (written.replace unreviewed "review = true")

/-- Where a building's plan lives, relative to the city. -/
def roadmapOf (addr : String) : String := s!"{addr}/Roadmap.md"

/-- Writes the first row of a building's plan, `1`, under the table's header.

A building is raised with a plan that has no rows, and the `plan` tool divides
rows that exist, so the first row is the person's to write. The row goes right
under the line of dashes that ends the table's header, in the six columns the
template names; a plan without exactly one such line is refused rather than
guessed at. -/
def layPlan (city : System.FilePath) (addr item : String) : IO Unit := do
  let plan := city / roadmapOf addr
  let lines := (← IO.FS.readFile plan).splitOn "
"
  match lines.filter (·.startsWith "|---") with
  | [rule] =>
    let row := s!"| 1 | {item} | 1 |  | Not started |  |"
    let laid := lines.flatMap fun line => if line == rule then [line, row] else [line]
    IO.FS.writeFile plan ("
".intercalate laid)
  | found =>
    throw <| IO.userError s!"{plan} has {found.length} header rules where the template has one"

end Sprawling.Acceptance
