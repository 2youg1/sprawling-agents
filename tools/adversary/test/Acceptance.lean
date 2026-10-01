-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling

/-!
# The acceptance world's two commands.

`just acceptance <archive>` is the only caller, and it calls them in this
order: `script` writes what the stand-in provider will play, the recipe starts
the stand-in on it, and `walk` drives the archive's binary through the city a
stranger raises. The recipe holds the order because it holds the processes:
the stand-in is started outside this directory, which hosts no server of its
own (`adversary-SPEC.md` section 13).

Unlike `adversary`, nothing here is skipped. The person asked for this run by
naming an archive, so a binary or a provider that is missing is a failure that
says which, rather than a green run that walked nothing.
-/

open Sprawling Sprawling.Acceptance

private def usage : String :=
  "usage: acceptance script <shelf> <script.json>\n" ++
  "       acceptance walk <shelf> <record.jsonl> <checklist.md>\n" ++
  "SPRAWLING_BIN names the binary to walk, SPRAWLING_PROVIDER the stand-in's URL."

/-- Writes the stand-in's script for the skills the archive's shelf holds. -/
private def writeScript (shelf out : System.FilePath) : IO UInt32 := do
  let skills ← shipped shelf
  if skills.isEmpty then
    throw <| IO.userError s!"{shelf} holds no skill, so the walk would pin and read nothing"
  IO.FS.writeFile out (script skills).compress
  return 0

/-- A variable this run cannot go on without. -/
private def required (name : String) : IO String := do
  match ← IO.getEnv name with
  | some value => if value.trimAscii.toString.isEmpty then absent else pure value.trimAscii.toString
  | none => absent
where
  absent := throw <| IO.userError s!"{name} is not set; run this through `just acceptance <archive>`"

/-- Walks the archive's binary, says where the walk stopped if it did, and
writes the person's checklist once it held. -/
private def walkWith (shelf record checklist : System.FilePath) : IO UInt32 := do
  let binary : System.FilePath := ← required "SPRAWLING_BIN"
  if !(← binary.pathExists) then
    throw <| IO.userError s!"SPRAWLING_BIN names no file: {binary}"
  let setting : Setting :=
    { door := ⟨binary⟩, url := ← required "SPRAWLING_PROVIDER", shelf
    , skills := ← shipped shelf, record }
  try
    let hosts ← walk setting (knownHosts setting.door)
    IO.println "the walk held"
    writeChecklist setting hosts checklist
    IO.println s!"the checklist is in {checklist}"
    return 0
  catch error =>
    IO.println s!"  FAIL  {error}"
    return 1

def main : List String → IO UInt32
  | ["script", shelf, out] => writeScript shelf out
  | ["walk", shelf, record, checklist] => walkWith shelf record checklist
  | _ => do
    IO.eprintln usage
    return 2
