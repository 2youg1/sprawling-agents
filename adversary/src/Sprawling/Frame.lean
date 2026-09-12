-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Lean.Data.Json

/-!
# The algebraic mirror of what a socket carries, and nothing else.

This module parses and does not judge. It knows the six frame classes
`channels::ServerFrame` spells and the shape of a refusal; it does not know
which refusal is owed, which is `Sprawling.Model`'s business.

An answer keeps its name and its body as it arrived. Fifteen queries exist and
the model drives four, so decoding all fifteen into structures would be fifteen
restatements of a shape that already has an authority in Rust. The name is
checked because a renamed answer is a wire change; the body is read only where
something asserts on it.
-/

namespace Sprawling

open Lean (Json)

/-- A stable error code, as the door prints it: `E_GATE_DENIED` and its 34
siblings.

Kept as text rather than as a closed enum on purpose. Mirroring `AxCode::ALL`
here would put a second copy of that table in this repository, and the property
this adversary asserts is that a particular refusal carries a particular code —
not that the set of codes is what some Lean file says it is. -/
structure Code where
  value : String
deriving BEq, DecidableEq, Inhabited

instance : ToString Code where
  toString c := s!"Code \"{c.value}\""

/-- What the server said when it took the connection. -/
structure Welcome where
  wire : Nat
  schema : String
  city : Option String
deriving BEq, Inhabited

/-- A refusal in the three parts `kernel::AxError` promises, plus the two fields
that travel with them. -/
structure Complaint where
  code : Code
  action : String
  subject : String
  recovery : String
  retriable : Bool
deriving BEq, Inhabited

instance : ToString Complaint where
  toString c := s!"Complaint {c.code} on {c.subject}"

/-- One line of history, as it was pushed. -/
structure Record where
  seq : Nat
  prev : String
  kind : String
  who : String
  run : String
  data : Json
deriving BEq, Inhabited

/-- Everything a server may send. -/
inductive Frame where
  | welcomed (welcome : Welcome)
  | happened (record : Record)
  /-- The answer's name, and its body unread. -/
  | answered (name : String) (body : Json)
  | refused (complaint : Complaint)
  /-- The run it belongs to, and the text so far. -/
  | streamed (run : String) (text : String)
  /-- One line of the process log: who it is for, and what it says.

  **A log is not history.** The line carries the ledger position it was written
  at rather than a sequence of its own, two lines can share that position, and a
  client that missed one has lost nothing — so nothing here reads it, and no
  property is written against it. It is carried because a city narrates its
  refusals on this channel as well as answering them on the other, and a reader
  looking at a failure wants the sentence. -/
  | logged (level : String) (line : String)
deriving BEq, Inhabited

instance : ToString Frame where
  toString
    | .welcomed w => s!"Welcomed wire {w.wire}"
    | .happened r => s!"Happened {r.kind} at seq {r.seq}"
    | .answered name _ => s!"Answered {name}"
    | .refused c => s!"Refused {c.code}"
    | .streamed run _ => s!"Streamed {run}"
    | .logged level line => s!"Logged {level}: {line}"

/-- The key/value pairs of a JSON object, in the order the reader folds them.

Written against the constructor rather than through `getObj?` because what the
callers below need is the *arity* of the envelope: a frame is one tagged object,
and a second tag is a wire change rather than a field this reader may ignore. -/
private def pairsOf (j : Json) : Option (List (String × Json)) :=
  match j with
  | .obj node => some node.toList
  | _ => none

private def expectObject (what : String) (j : Json) : Except String (List (String × Json)) :=
  match pairsOf j with
  | some pairs => .ok pairs
  | none => .error s!"{what} is not an object"

private def field (what : String) (pairs : List (String × Json)) (key : String) :
    Except String Json :=
  match pairs.find? (fun pair => pair.fst == key) with
  | some pair => .ok pair.snd
  | none => .error s!"{what} has no field {key}"

private def stringField (what : String) (pairs : List (String × Json)) (key : String) :
    Except String String := do
  (← field what pairs key).getStr?

private def natField (what : String) (pairs : List (String × Json)) (key : String) :
    Except String Nat := do
  (← field what pairs key).getNat?

private def boolField (what : String) (pairs : List (String × Json)) (key : String) :
    Except String Bool := do
  (← field what pairs key).getBool?

private def parseWelcome (body : Json) : Except String Welcome := do
  let pairs ← expectObject "Welcome" body
  let city :=
    match field "Welcome" pairs "city" with
    | .ok j => j.getStr?.toOption
    | .error _ => none
  return { wire := ← natField "Welcome" pairs "wire_v"
         , schema := ← stringField "Welcome" pairs "schema"
         , city }

private def parseComplaint (body : Json) : Except String Complaint := do
  let pairs ← expectObject "AxError" body
  return { code := ⟨← stringField "AxError" pairs "code"⟩
         , action := ← stringField "AxError" pairs "action"
         , subject := ← stringField "AxError" pairs "subject"
         , recovery := ← stringField "AxError" pairs "recovery"
         , retriable := ← boolField "AxError" pairs "retriable" }

private def parseRecord (body : Json) : Except String Record := do
  let pairs ← expectObject "EventRecord" body
  return { seq := ← natField "EventRecord" pairs "seq"
         , prev := ← stringField "EventRecord" pairs "prev"
         , kind := ← stringField "EventRecord" pairs "kind"
         , who := ← stringField "EventRecord" pairs "who"
         , run := ← stringField "EventRecord" pairs "run"
         , data := ← field "EventRecord" pairs "data" }

/-- An answer is one tagged object whose tag is the query's own name. -/
private def parseAnswer (body : Json) : Except String Frame := do
  match ← expectObject "Answer" body with
  | [(tag, inner)] => return .answered tag inner
  | fields => .error s!"an answer is one tagged object, not {fields.length}"

private def parseDelta (body : Json) : Except String Frame := do
  let pairs ← expectObject "Delta" body
  return .streamed (← stringField "Delta" pairs "run") (← stringField "Delta" pairs "text")

private def parseLog (body : Json) : Except String Frame := do
  let pairs ← expectObject "LogLine" body
  return .logged (← stringField "LogLine" pairs "level") (← stringField "LogLine" pairs "line")

private def parseFrame (value : Json) : Except String Frame := do
  match ← expectObject "ServerFrame" value with
  | [(tag, body)] =>
    match tag with
    | "welcome" => return .welcomed (← parseWelcome body)
    | "event" => return .happened (← parseRecord body)
    | "answer" => parseAnswer body
    | "refusal" => return .refused (← parseComplaint body)
    | "delta" => parseDelta body
    | "log" => parseLog body
    | other => .error s!"a frame class this adversary cannot read: {other}"
  | fields => .error s!"a frame is one tagged object, not {fields.length}"

/-- One line of the door's stdout.

A line this cannot read is an error rather than an ignored frame: the wire has
changed shape, and carrying on would test a program this adversary can no longer
read while reporting green. -/
def decodeFrame (raw : String) : Except String Frame := do
  parseFrame (← Json.parse raw)

/-- The addresses a `city_view` answer lists, sorted.

Sorted so that a property about which buildings stand does not accidentally
depend on the order the city happened to fold them in. Returns `none` when the
body is not a city answer at all, which the caller reports as a door that
changed shape rather than as a city with no buildings. -/
def cityBuildings (body : Json) : Option (List String) := do
  let listed ← (body.getObjVal? "buildings").toOption
  let entries ← listed.getArr?.toOption
  let addresses ← entries.toList.mapM fun entry =>
    (entry.getObjVal? "addr" >>= Json.getStr?).toOption
  return (addresses.toArray.qsort (· < ·)).toList

/-- The refusal in a batch of frames, if one is there. -/
def refusalOf (frames : List Frame) : Option Complaint :=
  frames.findSome? fun
    | .refused complaint => some complaint
    | _ => none

/-- The first record in a batch of frames whose kind is the one named. -/
def recordOf (kind : String) (frames : List Frame) : Option Record :=
  frames.findSome? fun
    | .happened record => if record.kind == kind then some record else none
    | _ => none

/-- The body of the first answer carrying the name asked for. -/
def answerOf (name : String) (frames : List Frame) : Option Json :=
  frames.findSome? fun
    | .answered found body => if found == name then some body else none
    | _ => none

end Sprawling
