-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Lean.Data.Json

/-!
# What the stand-in provider says, written before the city asks.

The acceptance world walks a city that calls a provider which answers. That
provider is not in this directory: it is the stand-in `tools/citysim` builds
(`citysim-SPEC.md` sections 8-10 and 8-13), started by `just acceptance` and
handed over as the URL in `SPRAWLING_PROVIDER`. What this module owns is the
script it plays — each run's replies, in the order the walk opens the runs —
because the script is the world's expectation stated as data: the walk asserts
what a city does when its model says exactly this.

**The replies are the OpenAI chat wire, written as values.** The stand-in reads
every reply through the city's own translation before it binds a port, so a
reply mistyped here is refused when the stand-in starts rather than reaching the
city as a mismatch nobody meant to test. Nothing here parses a request.

**A run is told apart by the call ids it carries back.** The stand-in places a
request in the run whose call ids it carries, and opens the next run of the
script for a request carrying none (`citysim-SPEC.md` section 3-11). So every
call id here is `callId run turn`, unique in the script, and the runs are listed
in the order the walk opens them: the walk waits for each run to freeze before
it sends the next, except for the two claimants, which it sends at once and
which are therefore written alike (`tools/adversary/Spec.lean` D6).
-/

namespace Sprawling.Acceptance

open Lean (Json JsonNumber)

/-- The one model the stand-in lists, and the id the walk attaches. -/
def standInModel : String := "stand-in-1"

/-- The building the walk raises and sends its work to.

One of the three names `Sprawling.cast` holds, so a red here reads like every
other report in this directory. -/
def building : String := "acme"

/-- The file the first run writes, relative to the city's root, and its bytes.

Written with `edit` because creating a file is the smallest piece of work whose
result a person can open; the walk reads the file back off the disk rather than
trusting the tool's own answer. -/
def notesPath : String := s!"{building}/notes.md"

def notesText : String := "written by a scripted resident\n"

/-- How many `status` calls the run the walk kills is given.

Enough that the run is still calling when the walk sees it working and pulls the
plug: one call answers in about ten milliseconds on a debug build, so a hundred
and twenty are over a second of a run in flight. -/
def statusCalls : Nat := 120

/-- How many `status` calls each claimant makes after it asks for the leaf.

The two claimants are sent at once, but each first waits for a tree of its own,
and the city places one tree after the other; the one that asks first has to be
still in flight when the other asks, or the second claim would be judged
against a run that already came home. Three hundred calls are a few seconds of
a debug build, more than one placement. -/
def holdCalls : Nat := 300

/-- The building the collaboration happens in: the second of `Sprawling.cast`,
raised by the walk and asked by the person to review its work. -/
def reviewed : String := "beta"

/-- The three rooms of the reviewed building: the one that divides the plan,
and the two that claim from it, offer work and check each other's. -/
def planner : String := s!"{reviewed}/planner"
def left : String := s!"{reviewed}/left"
def right : String := s!"{reviewed}/right"

/-- The row the person writes into the reviewed building's plan. -/
def planItem : String := "glaze the kiln"

/-- What the planner divides that row into. The plan numbers the children of
row `1` as `1.1` and `1.2` (`docs/templates/Roadmap.md`), which is what a person
reading the plan sees and what the claims below name. -/
def leaves : List String := ["wire the kiln", "glaze tests"]

def firstLeaf : String := "1.1"
def secondLeaf : String := "1.2"

/-- The file the re-dispatched run writes in its own tree and offers for
review, relative to the city's root, and its bytes. -/
def offeredPath : String := s!"{left}/kiln.md"

def offeredText : String := "the kiln is wired\n"

/-- What the resident who checks the offer says when it does not pass. -/
def refusedWhy : String := "the notes say nothing about the glaze"

/-- The call id of the `turn`-th reply of the `run`-th run: unique in the
script, so a request carrying it back names that run and that reply. -/
def callId (run turn : Nat) : String := s!"call-{run}-{turn}"

private def number (value : Nat) : Json := Json.num (JsonNumber.fromNat value)

/-- What a reply says it cost. The city records it and the walk asserts nothing
about it; it is here because a reply without usage is not a reply. -/
private def usage : Json :=
  Json.mkObj [("prompt_tokens", number 10), ("completion_tokens", number 5)]

/-- A reply that calls one tool with the arguments given. -/
def calling (run turn : Nat) (tool : String) (args : Json) : Json :=
  Json.mkObj
    [ ("choices", Json.arr #[Json.mkObj
        [ ("message", Json.mkObj
            [ ("role", .str "assistant")
            , ("tool_calls", Json.arr #[Json.mkObj
                [ ("id", .str (callId run turn))
                , ("type", .str "function")
                , ("function", Json.mkObj
                    [ ("name", .str tool)
                    -- The wire carries arguments as a string of JSON, not as
                    -- an object, which is the one shape here a hand would get
                    -- wrong.
                    , ("arguments", .str args.compress) ]) ]]) ])
        , ("finish_reason", .str "tool_calls") ]])
    , ("usage", usage) ]

/-- A reply that says something and calls nothing, which is how a run ends. -/
def saying (text : String) : Json :=
  Json.mkObj
    [ ("choices", Json.arr #[Json.mkObj
        [ ("message", Json.mkObj [("role", .str "assistant"), ("content", .str text)])
        , ("finish_reason", .str "stop") ]])
    , ("usage", usage) ]

/-- A run's replies, built for the place it takes in the script so that its
call ids are its own. -/
abbrev Run := Nat → List Json

/-- The first run: write a file, read every shipped skill by name, then stop.

One call per reply, so the order of the history is the order of this list. -/
def firstRun (skills : List String) : Run := fun run =>
  calling run 0 "edit"
      (Json.mkObj
        [ ("path", .str notesPath), ("base_version", .str "new")
        , ("old", .str ""), ("new", .str notesText) ])
    :: skills.zipIdx.map (fun (skill, index) =>
        calling run (index + 1) "read" (Json.mkObj [("path", .str skill)]))
    ++ [saying "the file is written and every skill was read"]

/-- The run the walk kills. If the city that comes back sends its conversation
on, it carries its own call ids back and ends on its own last line. -/
def interruptedRun : Run := fun run =>
  (List.range statusCalls).map (fun turn => calling run turn "status" (Json.mkObj []))
    ++ [saying "the city came back"]

/-- The work sent the morning after the crash. -/
def morningRun : Run := fun _ => [saying "the city came back"]

/-- Divides the person's row into the two leaves. -/
def plannerRun : Run := fun run =>
  [ calling run 0 "plan"
      (Json.mkObj
        [ ("action", .str "split"), ("node", .str "1")
        , ("parts", Json.arr (leaves.map Json.str).toArray) ])
  , saying "the plan is divided" ]

/-- Asks for the first leaf, then keeps calling for a while (`holdCalls`).
Both claimants are given this run: they open at once, so whichever reaches the
stand-in first, both say the same, and which of them holds the leaf is the
city's to decide. -/
def claimantRun : Run := fun run =>
  calling run 0 "plan" (Json.mkObj [("action", .str "claim"), ("node", .str firstLeaf)])
    :: (List.range holdCalls).map (fun turn => calling run (turn + 1) "status" (Json.mkObj []))
    ++ [saying "asked for the first leaf"]

/-- The work sent again: take the leaf still ready, write the file, offer it. -/
def offeringRun : Run := fun run =>
  [ calling run 0 "plan" (Json.mkObj [("action", .str "claim"), ("node", .str secondLeaf)])
  , calling run 1 "edit"
      (Json.mkObj
        [ ("path", .str offeredPath), ("base_version", .str "new")
        , ("old", .str ""), ("new", .str offeredText) ])
  , calling run 2 "pr" (Json.mkObj [("action", .str "open")])
  , saying "offered for review" ]

/-- Checks the offer on `branch` and says it does not pass. Written only once
the walk has read the branch from the history (`tools/adversary/Spec.lean` D7). -/
def checkerRun (branch : String) : Run := fun run =>
  [ calling run 0 "pr"
      (Json.mkObj
        [ ("action", .str "check"), ("branch", .str branch)
        , ("passed", Json.bool false), ("why", .str refusedWhy) ])
  , saying "checked, and it does not pass" ]

/-- The runs the walk opens, in the order it opens them. -/
def runsFor (skills : List String) : List Run :=
  [ firstRun skills, interruptedRun, morningRun ]

/-- A script of the runs given, each handed its place. -/
private def written (runs : List Run) : Json :=
  Json.mkObj
    [ ("face", .str "open_ai")
    , ("models", Json.arr #[.str standInModel])
    , ("runs", Json.arr (runs.zipIdx.map fun (run, index) =>
        Json.arr (run index).toArray).toArray) ]

/-- The whole script, as `citysim::WireScript::parse` reads it. -/
def script (skills : List String) : Json := written (runsFor skills)

/-- The same script with the run that checks the offer on `branch` after it:
it begins with every run the stand-in already holds, which is what lets the
stand-in take it when it reads the file again (`citysim-SPEC.md` section 3-15). -/
def scriptWithChecker (skills : List String) (branch : String) : Json :=
  written (runsFor skills ++ [checkerRun branch])

end Sprawling.Acceptance
