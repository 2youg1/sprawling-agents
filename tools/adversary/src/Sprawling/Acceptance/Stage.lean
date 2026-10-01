-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Layer

/-!
# The directory a stranger raises a city in, and what they edit by hand.

The acceptance world is a person with a fresh folder, the binary out of a
release archive, and a text editor. Everything that person does through the city
goes through `Sprawling.Door`; the two things they do with the editor — naming
the archive's `skills/` as a shelf, and admitting those skills in a building's
reading room — are written here, as the files say a person writes them
(`docs/getting-started.md`, "Tools, skills and MCP").

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

end Sprawling.Acceptance
