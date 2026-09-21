-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Ground

/-!
# The second world: a city that has been given a provider.

`Sprawling.Model` remembers a city nobody ever attached an endpoint to, and
`adversary-SPEC.md` section 3 records why: with no provider, every dispatch
stops at configuration and the model knows which refusal is owed. Attaching one
makes a different world, so it is written here rather than smuggled into that
one.

Two facts are driven through the door in this world, and each is a fact a
person enters on the settings page and the city then has to keep:

* **The URL a person pasted and the URL the city calls are one fact.** A
  provider's documentation prints the same endpoint three ways, so every
  spelling has to reach one registration; and the city's own answer, entered a
  second time, has to come back unchanged.
* **A model this city registered can be called.** The Anthropic wire refuses to
  write a request without an output ceiling, so a ceiling nobody stated has to
  be answered by the ladder rather than left absent.

Nothing here recomputes a normalisation, a ceiling or a ladder. Every assertion
is a relation between two readings the city itself wrote down, or the absence
of a refusal the door promises never to make.

**These verbs are read out of the history rather than off the socket, and that
is forced by what they cost.** A probe and an attachment each open a socket to
the provider, and the door returns only after the city has been silent for a
window — so waiting for their records on the connection would charge that
window on top of the call. The command is therefore sent, its refusal is still
read off the socket if one comes, and the record it produced is read where the
city put it. `Sprawling.Ground` already takes that position, and
`ARCHITECTURE.md` section 6 grants it to anyone who can read the directory.
-/

namespace Sprawling

open Lean (Json)

/-- Where this world points its endpoints: a port on this machine that nothing
listens on.

Outside the ground pool (47100–47115) on purpose, so a probe can never reach a
city this suite is serving. What is wanted from it is a connection refused
promptly: a name that does not resolve costs a resolver timeout on some
machines, and every action here is paid in wall clock. -/
def deadAuthority : String := "127.0.0.1:47199"

/-- The spelling this world attaches with, and the one its regression test
carries into the repository.

One definition because two would drift: the check below attaches it, and
`Sprawling.Regression` renders the same URL into the Rust test that remembers
what went wrong here. -/
def attachedUrl : String := s!"http://{deadAuthority}/v1"

/-- One endpoint, written every way a provider's documentation prints it.

The class is walked whole rather than sampled. It is finite and small, so
enumerating it is the universal statement itself, and a sample that happened to
draw one spelling twice would report a property nobody checked as checked.

The last entry carries no scheme, which is the form a person copies out of a
terminal; the city decides the scheme from whether the address is on this
machine. -/
def spellings (authority : String) : List String :=
  [ s!"http://{authority}/v1"
  , s!"http://{authority}/v1/"
  , s!"http://{authority}/v1/messages"
  , s!"http://{authority}"
  , authority ]

/-- Frames this build cannot read, in the three ways a caller gets one wrong.

A truncated object is not JSON at all; a command name nothing spells is JSON
whose grammar is wrong; a bare array is JSON whose shape is wrong. All three
are owed the same answer, which is why they are one list rather than three
checks. -/
def unreadable : List String :=
  [ "{\"command\":{\"dispatch\":"
  , "{\"command\":{\"no_such_verb\":{}}}"
  , "[]" ]

/-- The whole history as text, in the order the segments are named. -/
def Ground.text (ground : Ground) : IO String := do
  let stored ← ground.stored
  return String.join (stored.map fun segment => (String.fromUTF8? segment.snd).getD "")

/-- The history once the city has stopped adding to it, or once the budget
runs out.

Two readings a quarter second apart, because what a caller wants here is a
history that is not in the middle of being written: a city finishing its
start-up, or a run still reaching for a provider. Nothing about the content is
waited for, so this predicts no record and no ordering. -/
partial def Ground.stillText (ground : Ground) : Nat → IO String
  | 0 => ground.text
  | tries + 1 => do
    let before ← ground.text
    IO.sleep 250
    let after ← ground.text
    if before == after then return after else ground.stillText tries

/-- Every record the history holds, oldest first.

A line this cannot read is an error rather than a line skipped: the ledger has
changed shape, and carrying on would report a property against a history this
adversary can no longer read. -/
def Ground.records (ground : Ground) : IO (List Record) := do
  let lines := (← ground.text).splitOn "\n" |>.filter fun line =>
    !line.trimAscii.toString.isEmpty
  lines.mapM fun line =>
    match decodeRecord line with
    | .ok record => pure record
    | .error reason =>
      throw <| IO.userError <|
        s!"the history holds a line this adversary cannot read: {reason}\n" ++
        s!"        line: {line.take 400}"

/-- Waits until the history holds `wanted` records of one kind, and hands back
every record of that kind.

Polling the history rather than the socket, because the verbs of this world
answer after a round trip and the door charges a silence window for waiting.
What is waited for is the record the command was sent to produce, so nothing
about its content is predicted — a city that wrote it and a city that is still
writing it are told apart by counting, which is the one thing a caller who sent
`wanted` commands already knows.

Giving up is a failure rather than an empty answer: a command that produced no
record inside this budget is the finding, and an empty list handed back would
be asserted over vacuously. -/
partial def Ground.awaiting (ground : Ground) (kind : String) (wanted : Nat) :
    Nat → IO (List Record)
  | 0 =>
    throw <| IO.userError
      s!"the city wrote fewer than {wanted} {kind} record(s) before this gave up"
  | tries + 1 => do
    let found := (← ground.records).filter fun record => record.kind == kind
    if found.length ≥ wanted then return found
    IO.sleep 250
    ground.awaiting kind wanted tries

/-- How long this world waits for one record: a minute, in quarter seconds.

Generous, and paid only where a city is genuinely still working. A probe of an
address nothing listens on is refused by the kernel at once, and what remains
is the city building an HTTP client — which a debug build does far more slowly
than a shipped one, and this suite drives a debug build. -/
def recordTries : Nat := 240

/-- The `base_url` a record states, which is the one statement of what this
city will call. -/
def statedUrl (record : Record) : Option String :=
  (record.data.getObjVal? "base_url").toOption.bind fun url => url.getStr?.toOption

/-- Sends one command of this world and refuses to carry on if it was refused.

Silence is not acceptance anywhere in this directory, and it is not here
either: what is accepted here is that the door returned before the city had
finished, which is precisely what `quiet` means and why the record is then read
out of the history. -/
def Door.send (door : Door) (ground : Ground) (verb : Verb) : IO Unit := do
  match ← door.ask ground.port verb with
  | .denied complaint => throw <| IO.userError s!"{verb} was refused: {complaint}"
  | _ => pure ()

/-- The endpoint and the model this world registers.

An id no catalogue knows and no preset covers, which is the case the ladder's
bottom rung exists for: a person pasting a relay's own name for a model. Fixed
rather than drawn, so a counterexample names something a reader recognises
(`adversary-SPEC.md` section 14). -/
def relayName : String := "relay"

def unlistedModel : String := "opus-nine"

/-- What the `index`-th member of an equivalence class is registered under.

One name per spelling, because a second registration under one name is a
different question from a second spelling of one URL. -/
def nthName (index : Nat) : String := s!"{relayName}-{index}"

/-- Enters each URL as a probe and hands back what the city wrote down for it.

`already` is how many probes this ground has taken before now: the records come
back in the order the city wrote them, which is the order they were sent, so a
caller that probes twice in one city says how far along it is rather than
keeping a second count of its own. -/
def probedUrls (door : Door) (ground : Ground) (entered : List String) (already : Nat) :
    IO (List String) := do
  for (spelling, index) in entered.zipIdx do
    let ordinal := already + index
    door.send ground
      (.probeEndpoint (nthName ordinal) spelling .messages (idemKey (200 + ordinal)))
  let written ← ground.awaiting "endpoint_probed" (already + entered.length) recordTries
  (written.drop already).mapM fun record =>
    match statedUrl record with
    | some url => pure url
    | none =>
      throw <| IO.userError
        s!"an endpoint_probed record carries no base_url: {record.data.compress}"

/-- Enters each URL as an attachment and hands back what the city registered.

A probe and an attachment are two questions about one URL, and only the second
one decides what this city will call: a probe that normalised and an attachment
that did not would leave a settings page showing the right endpoint and a
dispatch reaching the wrong one. Every spelling carries the model id, because
an attachment that declared none is refused when the probe reaches nobody. -/
def attachedUrls (door : Door) (ground : Ground) (entered : List String) :
    IO (List String) := do
  for (spelling, index) in entered.zipIdx do
    door.send ground
      (.attachEndpoint (nthName index) spelling .messages [unlistedModel] (idemKey (220 + index)))
  let written ← ground.awaiting "endpoint_attached" entered.length recordTries
  written.mapM fun record =>
    match statedUrl record with
    | some url => pure url
    | none =>
      throw <| IO.userError
        s!"an endpoint_attached record carries no base_url: {record.data.compress}"

/-- The sentence the Anthropic wire writes when it will not call a model.

Matched as text because it is the door's own, printed to whoever is driving it;
what this adversary asserts is that a person who attached an endpoint and
picked a model never reads it. -/
def noCeilingSaid : String := "states no output ceiling"

/-- Whether a body of text carries that sentence. -/
def mentionsNoCeiling (said : String) : Bool := (said.splitOn noCeilingSaid).length > 1

/-- Attaches an endpoint nothing answers on and points the `main` tag at a
model no catalogue prices, and hands back the `model_selected` record.

The attachment survives a probe that reached nothing because the ids are named:
most compatible endpoints serve no model list, so a city that refused here
would keep a working provider out over an interface it never promised. That is
the product's own rule, and this world stands on it. -/
def givenAProvider (door : Door) (ground : Ground) (ceiling : Option Nat) : IO Record := do
  door.send ground
    (.attachEndpoint relayName attachedUrl .messages [unlistedModel] (idemKey 300))
  let _ ← ground.awaiting "endpoint_attached" 1 recordTries
  door.send ground (.selectModel relayName unlistedModel ceiling (idemKey 301))
  match ← ground.awaiting "model_selected" 1 recordTries with
  | chosen :: _ => return chosen
  | [] => throw <| IO.userError "choosing a model wrote no model_selected record"

/-- Which run each of the first `wanted` runs of this city was filed under.

Read from the envelope rather than from any payload: a record states the run it
belongs to, and that is the identity everything a run raises is named after. -/
def Ground.runsStarted (ground : Ground) (wanted : Nat) : IO (List String) := do
  let started ← ground.awaiting "run_started" wanted recordTries
  return started.map (·.run)

/-- What the `model_selected` record says this city registered the model with.

`none` is the record saying the ceiling is unstated, which is the reading the
ladder exists to make impossible: the Anthropic wire cannot write a request
from it. -/
def statedCeiling (chosen : Record) : Option Nat :=
  (chosen.data.getObjVal? "max_output_tokens").toOption.bind fun tokens =>
    tokens.getNat?.toOption

end Sprawling
