-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Acceptance.Walk

/-!
# The list a person works through by hand once the machine walk has held.

The walk proves what a machine can see from outside. What it cannot see — a
page that reads wrong, a command that does the right thing in the composer and
nothing in the palette, a skill that was read and used badly — is left to a
person, and this module writes the list they tick off.

**Every section is read from the one place that decides it, never listed
here.** The pages are the keys of the client's `BARE` table, the commands the
spellings of its `SLASH` table, the tools the catalogue the model was offered,
the skills the archive's shelf, and the providers the hosts the city itself
answers `known_hosts` with. A list kept in this file would be a second home for
each of them, and it would be the one that is wrong the day somebody adds a
page. Each heading names its source, so a reader who doubts a row knows where
to look.
-/

namespace Sprawling.Acceptance

open Sprawling
open Lean (Json)

/-- The client's page table and its command table, relative to the repository
root `just acceptance` runs in. Spelled with forward slashes because the same
text is printed under a heading, and every platform opens it. -/
def routeTable : String := "client/src/core/route.ts"

def slashTable : String := "client/src/core/slash.ts"

/-- The lines of a TypeScript table: those after the line that opens it, up to
the line that closes it. -/
private def tableLines (source opens closes : String) : List String :=
  ((source.splitOn "\n").dropWhile (fun line => !line.startsWith opens)).drop 1
    |>.takeWhile (fun line => line.trimAscii.toString != closes)

/-- The first double-quoted value on a line. -/
private def quoted (line : String) : Option String :=
  match line.splitOn "\"" with
  | _ :: value :: _ :: _ => some value
  | _ => none

/-- The page names of the `BARE` table: each key, without the empty one, in the
table's order.

The same reading `cargo xtask shots` takes of the same table
(`tools/xtask/xtask-SPEC.md` section 8-44), because it is the table the address
bar writes and reads; a comment line is skipped, since its words can hold a
colon. -/
def pagesIn (routes : String) : List String :=
  (tableLines routes "const BARE" "};").filterMap fun line =>
    let trimmed := line.trimAscii.toString
    if trimmed.startsWith "//" then none
    else
      match trimmed.splitOn ":" with
      | key :: _ :: _ =>
        let name := key.trimAscii.toString.replace "\"" ""
        if name.isEmpty then none else some name
      | _ => none

/-- What a command takes after its spelling.

`computed` is a grammar the table builds from another table — the effort words,
the page names — which this reader does not evaluate: it would be a second
account of that other table. -/
inductive Takes where
  | written (grammar : String)
  | computed

/-- Each command of the `SLASH` table: its spelling and what it takes after it. -/
def commandsIn (slash : String) : List (String × Takes) :=
  (tableLines slash "export const SLASH" "];").foldl (init := []) fun found line =>
    let trimmed := line.trimAscii.toString
    if trimmed.startsWith "spelling: " then
      match quoted trimmed with
      | some spelling => found ++ [(spelling, .written "")]
      | none => found
    else if trimmed.startsWith "grammar: " then
      let takes := if trimmed.startsWith "grammar: \"" then
          (quoted trimmed).map Takes.written |>.getD .computed
        else .computed
      match found.getLast? with
      | some (spelling, _) => found.dropLast ++ [(spelling, takes)]
      | none => found
    else found

/-- Each face of each host the city answered `known_hosts` with. -/
def facesIn (hosts : Json) : List (String × String × String) :=
  match (hosts.getObjVal? "hosts").toOption.bind (·.getArr?.toOption) with
  | none => []
  | some listed => listed.toList.flatMap fun host =>
    match (host.getObjValAs? String "host").toOption,
        (host.getObjVal? "faces").toOption.bind (·.getArr?.toOption) with
    | some name, some faces => faces.toList.filterMap fun face =>
        match (face.getObjValAs? String "dialect").toOption,
            (face.getObjValAs? String "base_url").toOption with
        | some dialect, some url => some (name, dialect, url)
        | _, _ => none
    | _, _ => []

/-- Asks the served city which hosts it knows, as the settings page does. -/
def knownHosts (door : Door) (ground : Ground) : IO Json := do
  match ← door.askRaw ground.port "{\"ask\":{\"ask_id\":1,\"query\":\"known_hosts\"}}" with
  | .accepted frames =>
    match answerOf "known_hosts" frames with
    | some body => return body
    | none => throw <| IO.userError "the city did not answer `known_hosts` with its hosts"
  | other => throw <| IO.userError s!"the city could not say which hosts it knows: {other}"

/-- What the checklist is written from. -/
structure Gathered where
  walked : List String
  pages : List String
  commands : List (String × Takes)
  tools : List String
  skills : List String
  faces : List (String × String × String)

/-- One section: a heading that names its source, and one unticked row per item.

A section with no rows says so, because an empty list under a heading reads as
"nothing to do" when it means the source was read as empty. -/
private def section' (heading : String) (rows : List String) : String :=
  let body :=
    if rows.isEmpty then "- [ ] this source was read as empty; find out why before ticking anything\n"
    else String.join (rows.map fun row => s!"- [ ] {row}\n")
  s!"\n## {heading}\n\n{body}"

/-- The checklist, as Markdown. -/
def render (gathered : Gathered) : String :=
  "# Hand-test checklist\n\n" ++
  "Written by `just acceptance` once the machine walk below held. Every unticked row is " ++
  "something a person does with the archive's binary and judges by eye. Each heading names " ++
  "the one place its rows are read from, so the list grows when the product does.\n" ++
  "\n## What the machine walked\n\n" ++
  String.join (gathered.walked.map fun step => s!"- [x] {step}\n") ++
  section' s!"Pages (`{routeTable}`, the `BARE` table)"
    (gathered.pages.map fun page =>
      s!"`#/{page}`: open it in the page the city serves, and read it against the city behind it") ++
  section' s!"Commands typed after `/` (`{slashTable}`, the `SLASH` table)"
    (gathered.commands.map fun (spelling, takes) =>
      let typed := match takes with
        | .written "" => s!"`{spelling}`"
        | .written grammar => s!"`{spelling} {grammar}`"
        | .computed => s!"`{spelling}` with each value the `/` menu offers after it"
      s!"{typed}: from the composer, and again from the Ctrl-K palette") ++
  section' "Tools a resident was offered (the first request the stand-in recorded)"
    (gathered.tools.map fun tool =>
      s!"`{tool}`: give a resident work that calls it, and read the result on the run's page") ++
  section' "Skills on the archive's shelf (`skills/`)"
    (gathered.skills.map fun skill =>
      s!"`{skill}`: pinned and read by name in the walk above; judge what a resident does with it") ++
  section' "Providers the city knows (its answer to `known_hosts`)"
    (gathered.faces.map fun (host, dialect, url) =>
      s!"`{host}`, `{dialect}` at `{url}`: one round, with a key you type in when you test")

/-- Reads every source and writes the checklist to `out`.

`hosts` is the city's own answer, asked while it was served; everything else is
read here, after the walk, from the repository and from the stand-in's record. -/
def writeChecklist (setting : Setting) (hosts : Json) (out : System.FilePath) : IO Unit := do
  let tools ← match ← requests setting.record with
    | first :: _ => pure (offered first)
    | [] => throw <| IO.userError "the stand-in recorded no request, so no catalogue to list"
  let gathered : Gathered :=
    { walked := walkedSteps setting
    , pages := pagesIn (← IO.FS.readFile routeTable)
    , commands := commandsIn (← IO.FS.readFile slashTable)
    , tools, skills := setting.skills, faces := facesIn hosts }
  IO.FS.writeFile out (render gathered)

end Sprawling.Acceptance
