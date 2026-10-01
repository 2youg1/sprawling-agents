-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Lean.Data.Json

/-!
# What the stand-in provider says, written before the city asks.

The acceptance world walks a city that calls a provider which answers. That
provider is not in this directory: it is the stand-in `tools/citysim` builds
(`citysim-SPEC.md` section 8-10), started by `just acceptance` and handed over as
the URL in `SPRAWLING_PROVIDER`. What this module owns is the script it plays —
the replies, in the order the city's requests arrive — because the script is the
world's expectation stated as data: the walk asserts what a city does when its
model says exactly this.

**The replies are the OpenAI chat wire, written as values.** The stand-in reads
every reply through the city's own translation before it binds a port, so a
reply mistyped here is refused when the stand-in starts rather than reaching the
city as a mismatch nobody meant to test. Nothing here parses a request.

**The order is the walk's order, and the walk is sequential.** A reply is spent
by whichever request arrives next, so each run the walk dispatches is waited for
before the next one goes out; the one run the walk does not wait for is the one
it kills, and that run's replies are written so that whoever spends the rest of
them ends the same way (`statusCalls`).
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
and twenty are over a second of a run in flight. Every one of them is the same
read-only call, so the run that spends whatever the killed one left — the work
dispatched after the city comes back — makes the same calls and ends on the
same line. -/
def statusCalls : Nat := 120

/-- How many closing lines follow the `status` calls.

More than one, because a city that came back and also picked up the killed run
would have two runs reaching for the last line; a second run that found the
script spent would stop on a refusal rather than on its own last line. -/
def closingLines : Nat := 3

private def number (value : Nat) : Json := Json.num (JsonNumber.fromNat value)

/-- What a reply says it cost. The city records it and the walk asserts nothing
about it; it is here because a reply without usage is not a reply. -/
private def usage : Json :=
  Json.mkObj [("prompt_tokens", number 10), ("completion_tokens", number 5)]

/-- A reply that calls one tool with the arguments given. -/
def calling (index : Nat) (tool : String) (args : Json) : Json :=
  Json.mkObj
    [ ("choices", Json.arr #[Json.mkObj
        [ ("message", Json.mkObj
            [ ("role", .str "assistant")
            , ("tool_calls", Json.arr #[Json.mkObj
                [ ("id", .str s!"call-{index}")
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

/-- The first run: write a file, read every shipped skill by name, then stop.

One call per reply, so the order of the history is the order of this list. -/
def firstRun (skills : List String) : List Json :=
  calling 0 "edit"
      (Json.mkObj
        [ ("path", .str notesPath), ("base_version", .str "new")
        , ("old", .str ""), ("new", .str notesText) ])
    :: skills.zipIdx.map (fun (skill, index) =>
        calling (index + 1) "read" (Json.mkObj [("path", .str skill)]))
    ++ [saying "the file is written and every skill was read"]

/-- The run the walk kills, and the run that finishes what it left. -/
def interruptedRun : List Json :=
  (List.range statusCalls).map (fun index => calling (1000 + index) "status" (Json.mkObj []))
    ++ List.replicate closingLines (saying "the city came back")

/-- The whole script, as `citysim::WireScript::parse` reads it. -/
def script (skills : List String) : Json :=
  Json.mkObj
    [ ("face", .str "open_ai")
    , ("models", Json.arr #[.str standInModel])
    , ("replies", Json.arr (firstRun skills ++ interruptedRun).toArray) ]

end Sprawling.Acceptance
