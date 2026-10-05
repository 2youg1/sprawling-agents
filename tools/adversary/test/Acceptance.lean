-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling

/-!
# The acceptance world's commands.

`just acceptance <archive>` calls the archive commands in this
order: `script` writes what the stand-in provider will play, the recipe starts
the stand-in on it, and `walk` drives the archive's binary through the city a
stranger raises. The recipe holds the order because it holds the processes:
the stand-in is started outside this directory, which hosts no server of its
own (`tools/adversary/Spec.lean` section 13).

Unlike `adversary`, nothing here is skipped. The person asked for this run by
naming an archive, so a binary or a provider that is missing is a failure that
says which, rather than a green run that walked nothing.

The fresh/runtime matrix also calls client admission, the browser audit,
version admission and gauge against the archive binary; client-ui uses the
same Stage.serving as the walk and never starts a second app server.
-/

open Sprawling Sprawling.Acceptance
open Lean (Json)

private def usage : String :=
  "usage: acceptance script <shelf> <script.json>\n" ++
  "       acceptance walk <shelf> <script.json> <record.jsonl> <checklist.md>\n" ++
  "       acceptance client\n" ++
  "       acceptance client-ui <script> <output>\n" ++
  "       acceptance gauge <output>\n" ++
  "SPRAWLING_BIN names the binary to walk, SPRAWLING_PROVIDER the stand-in's URL,
" ++
  "SPRAWLING_LAUNCHER, where set, the launcher that closes each city in order on Windows."

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

/-- The launcher the recipe names on Windows, or none (tools/adversary/Spec.lean D8). -/
private def launcher : IO (Option System.FilePath) := do
  match (← IO.getEnv "SPRAWLING_LAUNCHER").map (·.trimAscii.toString) with
  | some named => if named.isEmpty then return none else return some named
  | none => return none

/-- Walks the archive's binary, says where the walk stopped if it did, and
writes the person's checklist once it held. -/
private def walkWith (shelf script record checklist : System.FilePath) : IO UInt32 := do
  let binary : System.FilePath := ← required "SPRAWLING_BIN"
  if !(← binary.pathExists) then
    throw <| IO.userError s!"SPRAWLING_BIN names no file: {binary}"
  let setting : Setting :=
    { door := { binary, launcher := ← launcher }, url := ← required "SPRAWLING_PROVIDER", shelf
    , skills := ← shipped shelf, script, record }
  try
    let hosts ← walk setting (knownHosts setting.door)
    IO.println "the walk held"
    writeChecklist setting hosts checklist
    IO.println s!"the checklist is in {checklist}"
    return 0
  catch error =>
    IO.println s!"  FAIL  {error}"
    return 1

/-- Serves one temporary city for client admission, closes it on every result. -/
private def deliveredClient (after : Ground → IO Unit) : IO UInt32 := do
  let door : Door := { binary := ← required "SPRAWLING_BIN", launcher := ← launcher }
  let stage ← Stage.raise door
  try
    stage.serving door .closedInOrder fun ground => do
      Step.runAll ground [clientDelivered]
      after ground
    return 0
  catch error =>
    IO.eprintln s!"  FAIL  {error}"
    return 1
  finally
    stage.discard

/-- The browser receives this serving's port; it does not start another server. -/
private def clientPixels (script out : String) (ground : Ground) : IO Unit := do
  let result ← IO.Process.output
    { cmd := "bun", args := #[script, s!"{ground.port}", out] }
  IO.print result.stdout
  IO.eprint result.stderr
  ensure (result.exitCode == 0) s!"the production client browser audit failed ({result.exitCode})"

/-- Measures actual successful, failed and cancelled children and keeps the raw readings. -/
private def gaugeCases (out : System.FilePath) : IO UInt32 := do
  let binary ← required "SPRAWLING_BIN"
  IO.FS.createDirAll out
  let samples := 20
  let cancelled ← match ← IO.getEnv "RUNNER_OS" with
    | some "Windows" => pure #["powershell", "-NoProfile", "-Command", "Stop-Process -Id $PID -Force"]
    | _ => pure #["sh", "-c", "kill -TERM $$"]
  for (name, args, failed) in
      [("success", #[binary, "status"], 0), ("failure", #[binary, "call", "not-json"], samples),
       ("cancelled-child", cancelled, samples)] do
    let command := #["gauge", "--samples", s!"{samples}", "--"] ++ args
    let result ← IO.Process.output { cmd := binary, args := command }
    IO.FS.writeFile (out / s!"{name}.jsonl") result.stdout
    IO.FS.writeFile (out / s!"{name}.stderr") result.stderr
    IO.FS.writeFile (out / s!"{name}.command.json")
      (Json.arr (command.map Json.str)).compress
    ensure (result.exitCode == 0) s!"gauge did not complete {name}: {result.stderr}"
    let lines ← (result.stdout.splitOn "
" |>.filter (!·.trimAscii.toString.isEmpty)).mapM fun line =>
      match Json.parse line with
      | .ok reading => pure reading
      | .error why => throw <| IO.userError s!"gauge {name} returned invalid JSON: {why}"
    let runs := lines.filter fun line => (line.getObjValAs? String "line").toOption == some "run"
    ensure (runs.length == samples) s!"gauge {name} omitted workload samples"
    let failures := runs.filter fun run => (run.getObjValAs? Nat "exit").toOption != some 0
    ensure (failures.length == failed) s!"gauge {name} reported unexpected child outcomes"
    match lines.find? (fun line => (line.getObjValAs? String "line").toOption == some "spread") with
    | none => throw <| IO.userError s!"gauge {name} omitted spread"
    | some spread =>
      ensureEq (some samples) (spread.getObjValAs? Nat "samples").toOption "spread omitted samples"
      ensureEq (some failed) (spread.getObjValAs? Nat "failed").toOption "spread erased failed samples"
    IO.println s!"  ok    gauge {name}: {samples} samples, {failed} failed"
  return 0

/-- Checks the installed binary against the caller's Cargo identity. -/
private def installedVersion (name version : String) : IO UInt32 := do
  let door : Door := { binary := ← required "SPRAWLING_BIN" }
  let said ← door.status
  match said.splitOn "
" with
  | headline :: _ =>
    if headline.startsWith s!"{name} {version} (" then
      IO.println headline
      return 0
    else
      IO.eprintln s!"expected {name} {version}; status said: {headline}"
      return 1
  | [] =>
    IO.eprintln "status printed no version line"
    return 1

def main : List String → IO UInt32
  | ["client"] => deliveredClient (fun _ => pure ())
  | ["client-ui", script, out] => deliveredClient (clientPixels script out)
  | ["gauge", out] => gaugeCases out
  | ["version", name, version] => installedVersion name version
  | ["script", shelf, out] => writeScript shelf out
  | ["walk", shelf, script, record, checklist] => walkWith shelf script record checklist
  | _ => do
    IO.eprintln usage
    return 2
