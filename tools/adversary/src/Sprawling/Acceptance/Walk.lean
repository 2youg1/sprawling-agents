-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Provider
import Sprawling.Acceptance.Script
import Sprawling.Acceptance.Stage

/-!
# The walk: a stranger's first day with a city, and the morning after a crash.

Every step is something a person does through the binary they downloaded, and
every assertion is read back where the city wrote it — a file on the disk, a
record in the one history, an answer to a query, a request the stand-in
provider recorded. Nothing is recomputed: a step that needed to know how the
city decides something would be a second account of that decision.

The steps run in order and stop at the first that breaks, because each one
stands on the city the steps before it left: a building that was never raised
cannot be worked in, and a walk that went on would report one cause many times.

The steps of three parts live here, with what every part reads the same way:

* **the first day** — the city answers, the stand-in is attached and its model
  chosen, a building is raised, its reading room admits the shipped skills, and
  work sent there runs to its end;
* **the crash** — work is sent and the process is killed while that run is
  still calling tools;
* **the morning after** — what the killed city left verifies, the city serves
  again, and new work runs to its end.

The fourth part, the collaboration, is `Sprawling.Acceptance.Collaboration`, and
the order the parts are served in is `Sprawling.Acceptance.Servings`.
-/

namespace Sprawling.Acceptance

open Sprawling
open Lean (Json)

/-- What the walk was handed: the binary, the stand-in's URL, the archive's
shelf with the skills on it, the script the stand-in plays, which the walk
appends a run to once the city has named what that run needs
(`tools/adversary/Spec.lean` D7), and the file the stand-in records into. -/
structure Setting where
  door : Door
  url : String
  shelf : System.FilePath
  skills : List String
  script : System.FilePath
  record : System.FilePath

/-- One step: what a person would say they did, and the check that it worked. -/
structure Step where
  name : String
  walk : Ground → IO Unit

/-- Runs steps in order against one served city, printing each as it holds.

The first failure is thrown with the step's name in front of it, so a red
names where the walk stopped rather than only why. -/
def Step.runAll (ground : Ground) : List Step → IO Unit
  | [] => pure ()
  | step :: rest => do
    let started ← IO.monoMsNow
    try step.walk ground
    catch error => throw <| IO.userError s!"{step.name}\n        {error}"
    IO.println s!"  ok    {step.name} ({(← IO.monoMsNow) - started} ms)"
    Step.runAll ground rest

/-- A string field of a record's payload. -/
def field (record : Record) (key : String) : Option String :=
  (record.data.getObjVal? key).toOption.bind fun value => value.getStr?.toOption

/-- Waits until the history holds a record of `kind` that `holds`, and hands it
back; giving up is a failure, for the reason `Ground.awaiting` gives. -/
partial def awaitingOne (ground : Ground) (kind : String) (holds : Record → Bool)
    (what : String) : Nat → IO Record
  | 0 => throw <| IO.userError s!"the city wrote no {kind} record {what} before this gave up"
  | tries + 1 => do
    match (← ground.records).find? (fun record => record.kind == kind && holds record) with
    | some found => return found
    | none =>
      IO.sleep 250
      awaitingOne ground kind holds what tries

/-- The buildings a `city_view` answer lists. -/
def listed (door : Door) (ground : Ground) : IO (List String) := do
  match ← door.ask ground.port .cityView with
  | .accepted frames =>
    match (answerOf "city" frames).bind cityBuildings with
    | some buildings => return buildings
    | none => throw <| IO.userError "the city did not answer `city_view` with a city"
  | other => throw <| IO.userError s!"the city could not be read: {other}"

/-- Sends work to a room of the building, the way the composer does. -/
def dispatching (session : String) (key : Nat) : Verb :=
  .dispatch building session (idemKey key)

/-- Every request the stand-in recorded, oldest first, as the request bodies it
was sent, parsed. A `GET` has an empty body and is left out. -/
def requests (record : System.FilePath) : IO (List Json) := do
  let lines := (← IO.FS.readFile record).splitOn "\n" |>.filter fun line =>
    !line.trimAscii.toString.isEmpty
  let exchanges ← lines.mapM fun line =>
    match Json.parse line with
    | .ok exchange => pure exchange
    | .error why => throw <| IO.userError s!"the stand-in recorded a line this cannot read: {why}"
  return exchanges.filterMap fun exchange =>
    match (exchange.getObjValAs? String "body").toOption with
    | some body => if body.isEmpty then none else (Json.parse body).toOption
    | none => none

/-- The names of the tools one request offered the model. -/
def offered (request : Json) : List String :=
  match (request.getObjVal? "tools").toOption.bind (·.getArr?.toOption) with
  | some tools => tools.toList.filterMap fun tool =>
      ((tool.getObjVal? "function").toOption.bind fun function =>
        (function.getObjValAs? String "name").toOption)
  | none => []

/-! ## The first day -/

/-- The building `init` raises with every city. -/
def hall : String := "hall"

/-- What stands once the walk has raised its building, in the order
`cityBuildings` answers. -/
def standing : List String := ([building, hall].toArray.qsort (· < ·)).toList

/-- `init` lays the hall down, and nothing else stands until a person raises it. -/
def answers (setting : Setting) : Step :=
  { name := "a raised city answers with the hall it was raised with"
  , walk := fun ground => do
      ensureEq [hall] (← listed setting.door ground) "the city stood up with other buildings" }

/-- The stand-in is attached the way the settings page attaches an endpoint, and
the city stores the URL it was given. -/
def attached (setting : Setting) : Step :=
  { name := "the stand-in provider is attached and its model chosen"
  , walk := fun ground => do
      setting.door.send ground
        (.attachEndpoint "stand-in" setting.url .chat [standInModel] (idemKey 400))
      let records ← ground.awaiting "endpoint_attached" 1 recordTries
      ensureEq [some setting.url] (records.map statedUrl)
        "the city registered the stand-in under another URL"
      setting.door.send ground (.selectModel "stand-in" standInModel none (idemKey 401))
      let chosen ← ground.awaiting "model_selected" 1 recordTries
      ensureEq [some standInModel] (chosen.map (field · "model"))
        "the city chose another model than the one the stand-in lists" }

def raised (setting : Setting) : Step :=
  { name := "a building is raised and the city lists it"
  , walk := fun ground => do
      setting.door.send ground (.createBuilding building .minimal (idemKey 402))
      ensureEq standing (← listed setting.door ground)
        "the city did not list the building it raised" }

/-- The person's hand edit, as a step: it can fail when the template changed,
and that is a finding about what a person is told to edit. -/
def admitted (setting : Setting) : Step :=
  { name := "the building's reading room admits every shipped skill"
  , walk := fun ground => admit ground.city building setting.skills }

/-- Work runs to the end the script gives it, and its file is on the disk. -/
def worked (setting : Setting) : Step :=
  { name := "work sent to the building runs to its end and leaves its file"
  , walk := fun ground => do
      setting.door.send ground (dispatching "one" 403)
      let frozen ← ground.awaiting "run_frozen" 1 recordTries
      ensureEq [some "done"] (frozen.map (field · "completion"))
        "the first run did not end on its own last line"
      ensureEq notesText (← IO.FS.readFile (ground.city / notesPath))
        s!"{notesPath} does not hold what the run wrote" }

/-- The skills a run started with are the ones on the shelf, and each one the
script read by name reached the model as the skill's own text. -/
def pinned (setting : Setting) : Step :=
  { name := "every shipped skill is pinned, and read by name it reaches the model"
  , walk := fun ground => do
      let started ← ground.awaiting "run_started" 1 recordTries
      let pins := started.flatMap fun record =>
        match (record.data.getObjVal? "skills").toOption.bind (·.getArr?.toOption) with
        | some skills => skills.toList.filterMap fun pin => (pin.getObjValAs? String "name").toOption
        | none => []
      ensureEq setting.skills ((pins.toArray.qsort (· < ·)).toList)
        "the run was not pinned to the skills the archive ships"
      let sent := String.join ((← requests setting.record).map (·.compress))
      for skill in setting.skills do
        ensure ((sent.splitOn s!"name: {skill}").length > 1)
          s!"the skill {skill} was read by name and its text never reached the model" }

/-- The catalogue the model was offered holds every tool the script calls. -/
def catalogued (setting : Setting) : Step :=
  { name := "the model is offered every tool the script calls"
  , walk := fun _ => do
      match ← requests setting.record with
      | [] => ensure false "the stand-in recorded no request a model was sent"
      | first :: _ =>
        let tools := offered first
        for tool in ["edit", "read", "status"] do
          ensure (tools.contains tool) s!"the model was not offered `{tool}`; it was offered {tools}" }

/-- Two readings of one fact: what the city answers, and what its history
says was created. -/
def agreed (setting : Setting) : Step :=
  { name := "the city lists exactly the buildings its history created"
  , walk := fun ground => do
      let created := (← ground.records).filterMap fun record =>
        if record.kind == "building_created" then field record "addr" else none
      ensureEq ((created.toArray.qsort (· < ·)).toList) (← listed setting.door ground)
        "the city's answer and its history disagree about what stands" }

/-- The history verifies, by the binary's own offline verifier. -/
def verified (setting : Setting) (when : String) : Step :=
  { name := s!"the history verifies {when}"
  , walk := fun ground => do
      match ← setting.door.verify ground.ledger with
      | .ok _ => pure ()
      | .error why => ensure false s!"the history did not verify: {why}" }

def firstDay (setting : Setting) : List Step :=
  [ answers setting, attached setting, raised setting, admitted setting, worked setting
  , pinned setting, catalogued setting, agreed setting, verified setting "after the first day" ]

/-! ## The crash -/

/-- How many tool results the interrupted run has written when the walk pulls
the plug: enough that the run is plainly in flight, few enough that the
`status` calls are far from spent (`statusCalls`). -/
def inFlight : Nat := 3

/-- What the kill is called in a report. -/
def interruptedStep : String := "a city is killed while a run is calling tools"

/-- Sends work, waits until its run is calling tools, and kills the city.

The work is sent on a task of its own, because the door returns only once the
city has gone quiet, and a run that is still calling never does. Leaving the
serving is what kills the process — `Serving.hangUp` terminates it — so the
kill lands between two of the run's writes rather than after its last one. -/
def interrupted (setting : Setting) (stage : Stage) : IO Unit := do
  let started ← IO.monoMsNow
  let (ground, sending) ← stage.serving setting.door .killed fun ground => do
    let sending ← IO.asTask (setting.door.ask ground.port (dispatching "two" 404)) .dedicated
    match (← ground.runsStarted 2)[1]? with
    | some doomed => waitInFlight ground doomed recordTries
    | none => throw <| IO.userError "the city started no run for the work sent second"
    return (ground, sending)
  -- The dispatch's own client lost its city mid-answer, which is the point;
  -- what it printed says nothing about the product, so it is waited for and
  -- set aside.
  let _ ← IO.wait sending
  let frozen := (← ground.records).filter (·.kind == "run_frozen")
  ensure (frozen.length == 1)
    s!"the interrupted run finished before the city was killed; give it more than \
      {statusCalls} calls"
  IO.println s!"  ok    {interruptedStep} ({(← IO.monoMsNow) - started} ms)"
where
  waitInFlight (ground : Ground) (run : String) : Nat → IO Unit
    | 0 => throw <| IO.userError "the interrupted run never called a tool"
    | tries + 1 => do
      let results := (← ground.records).filter fun record =>
        record.kind == "tool_result" && record.run == run
      if results.length ≥ inFlight then return
      IO.sleep 20
      waitInFlight ground run tries

/-! ## The morning after -/

/-- New work after the crash runs to its end: the run that started after the
city came back is the one whose freezing is waited for, whatever the city did
with the run it lost. -/
def resumed (setting : Setting) : Step :=
  { name := "the city serves again and new work runs to its end"
  , walk := fun ground => do
      ensureEq standing (← listed setting.door ground)
        "the city came back with other buildings"
      setting.door.send ground (dispatching "three" 405)
      let runs ← ground.runsStarted 3
      match runs.getLast? with
      | none => ensure false "the city started no run for the work sent after the crash"
      | some third =>
        let frozen ← awaitingOne ground "run_frozen" (·.run == third)
          "for the work sent after the crash" recordTries
        ensureEq (some "done") (field frozen "completion")
          "the work sent after the crash did not end on its own last line" }

def morningAfter (setting : Setting) : List Step :=
  [ verified setting "that a killed city left behind", resumed setting
  , verified setting "after the morning's work" ]

end Sprawling.Acceptance
