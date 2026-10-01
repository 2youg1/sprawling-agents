-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Acceptance.Walk

/-!
# The collaboration: a plan divided, one leaf claimed twice, work sent again,
and a review that does not pass.

Everything happens in a second building, `reviewed`, which the person asks to
review its work and whose plan they start with one row. A resident divides the
row into two leaves; two runs sent at once both ask for the first leaf, and the
history shows one holder; the work is sent again to a room that takes the leaf
still ready, writes a file in its own tree and offers it; another resident
checks the offer and says it does not pass, and the file never reaches the
building.

Every assertion is read back where the city wrote it, as in the rest of the
walk. The one thing the walk learns from the city before it can go on is the
branch the offer was made on: it is named after a digest of the room, which
this directory does not predict, so the walk reads it from the history and
appends the run that checks it to the stand-in's script
(`tools/adversary/Spec.lean` D7).
-/

namespace Sprawling.Acceptance

open Sprawling

/-- How many runs the city has started so far. -/
def startedSoFar (ground : Ground) : IO Nat :=
  return ((← ground.records).filter (·.kind == "run_started")).length

/-- Waits until `run` has frozen, and requires that it ended on its own last
line: a run the stand-in refused, or one a tool broke, ends otherwise. -/
def endedOwnLine (ground : Ground) (run what : String) : IO Unit := do
  let frozen ← awaitingOne ground "run_frozen" (·.run == run) s!"for {what}" recordTries
  ensureEq (some "done") (field frozen "completion") s!"{what} did not end on its own last line"

/-- Sends one piece of work, waits for the run it started to end on its own
last line, and names that run. Every other run the walk sends has frozen by
then, so the run started next is this one. -/
def sentAndEnded (setting : Setting) (ground : Ground) (room session : String) (key : Nat) :
    IO String := do
  let before ← startedSoFar ground
  setting.door.send ground (.dispatch room session (idemKey key))
  let started ← ground.awaiting "run_started" (before + 1) recordTries
  match started[before]? with
  | some record =>
    endedOwnLine ground record.run s!"the work sent to {room}"
    return record.run
  | none => throw <| IO.userError s!"the city started no run for the work sent to {room}"

/-- The records of `kind` whose payload says `value` under `key`. -/
def recordsWhere (ground : Ground) (kind key value : String) : IO (List Record) :=
  return (← ground.records).filter fun record =>
    record.kind == kind && field record key == some value

/-- The person raises the second building, asks it to review its work, and
writes the first row of its plan. -/
def reviewRaised (setting : Setting) : Step :=
  { name := "a second building asks for review and its plan is given a row"
  , walk := fun ground => do
      setting.door.send ground (.createBuilding reviewed .minimal (idemKey 406))
      askForReview ground.city reviewed
      layPlan ground.city reviewed planItem }

/-- A resident divides the row, and the history says into what. -/
def divided (setting : Setting) : Step :=
  { name := "a resident divides the plan's row into two leaves"
  , walk := fun ground => do
      let _ ← sentAndEnded setting ground planner "plan" 407
      let splits ← recordsWhere ground "roadmap_split" "node" "1"
      let children := splits.map fun record =>
        match (record.data.getObjVal? "children").toOption.bind (·.getArr?.toOption) with
        | some named => named.toList.filterMap (·.getStr?.toOption)
        | none => []
      ensureEq [leaves] children "the plan's row was not divided into the two leaves" }

/-- Two runs sent at once ask for the same leaf, and one of them holds it.

Both are sent before either is waited for, each on a task of its own because
the door returns only once the city has gone quiet. They are given the same
replies (`claimantRun`), and each keeps calling for a while after it asks
(`holdCalls`), so whichever asked first still holds the leaf when the other
asks: the second is refused by the run in flight rather than let through by a
run that already came home. -/
def claimedOnce (setting : Setting) : Step :=
  { name := "two runs sent at once ask for one leaf, and one of them holds it"
  , walk := fun ground => do
      let before ← startedSoFar ground
      let sending ← [left, right].zipIdx.mapM fun (room, index) =>
        IO.asTask (setting.door.ask ground.port (.dispatch room "claim" (idemKey (408 + index))))
          .dedicated
      for sent in sending do
        match ← IO.wait sent with
        | .ok (.denied complaint) => ensure false s!"a claimant's work was refused: {complaint}"
        | .ok _ => pure ()
        | .error why => throw why
      let started ← ground.awaiting "run_started" (before + 2) recordTries
      for record in started.drop before do
        endedOwnLine ground record.run "a claimant"
      let claims ← recordsWhere ground "roadmap_claimed" "node" firstLeaf
      ensureEq 1 claims.length
        s!"the leaf {firstLeaf} was claimed by more than one run, or by none; if the second \
          claim came after the first run came home, give the claimants more than {holdCalls} calls" }

/-- The work is sent again: the run takes the leaf still ready, writes a file
in its own tree and offers it, and the file is not the building's yet. -/
def offeredForReview (setting : Setting) : Step :=
  { name := "the work is sent again, takes the leaf still ready, and is offered for review"
  , walk := fun ground => do
      let _ ← sentAndEnded setting ground left "again" 410
      ensureEq 1 (← recordsWhere ground "roadmap_claimed" "node" secondLeaf).length
        s!"the leaf {secondLeaf} was not claimed once"
      ensureEq 1 ((← ground.records).filter (·.kind == "pr_opened")).length
        "the work was not offered for review once"
      ensure (!(← (ground.city / offeredPath).pathExists))
        s!"{offeredPath} reached the building before anyone checked it" }

/-- Another resident checks the offer and says it does not pass; the offer is
refused under its branch with that reason, and the file stays out. -/
def refused (setting : Setting) : Step :=
  { name := "another resident checks the offer, and it does not pass"
  , walk := fun ground => do
      let opened := (← ground.records).filter (·.kind == "pr_opened")
      let some branch := opened.head?.bind (field · "branch")
        | ensure false "the history holds no offer with a branch to check"
      IO.FS.writeFile setting.script (scriptWithChecker setting.skills branch).compress
      let _ ← sentAndEnded setting ground right "check" 411
      let rejected ← recordsWhere ground "pr_rejected" "branch" branch
      ensureEq [some refusedWhy] (rejected.map (field · "why"))
        s!"the offer on {branch} was not refused once with the checker's reason"
      ensureEq 0 ((← ground.records).filter (·.kind == "pr_merged")).length
        "an offer that did not pass was merged"
      ensure (!(← (ground.city / offeredPath).pathExists))
        s!"{offeredPath} reached the building although its check did not pass" }

def collaboration (setting : Setting) : List Step :=
  [ reviewRaised setting, divided setting, claimedOnce setting, offeredForReview setting
  , refused setting, verified setting "after the collaboration" ]

end Sprawling.Acceptance
