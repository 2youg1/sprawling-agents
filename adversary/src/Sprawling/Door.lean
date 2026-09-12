-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Frame

/-!
# The only place that knows a binary exists.

Everything this adversary learns, it learns by running the program an agent
runs, with the arguments an agent types, and reading the two streams an agent
reads. There is no linking, no FFI, and no shared type: what cannot be reached
through this module cannot be tested here, which is the point.

ARCHITECTURE section 8 says the wire is the whole API and that a second client
writes against it. `sprawling call` is that second client; this module is a
third one, written outside the repository to attack rather than to use.
-/

namespace Sprawling

open Lean (Json)

/-- A binary that has already been built. -/
structure Door where
  binary : System.FilePath
deriving Inhabited

/-- Where a city is listening. -/
structure Port where
  number : Nat
deriving BEq, DecidableEq, Inhabited

instance : ToString Port where
  toString p := toString p.number

/-- The deduplication key every state-changing command owns.

Minted here rather than in the model, and never predicted: the shape is the
door's (`idem1-` plus 32 lowercase hex digits) and what this adversary asserts
about it is that distinct actions carry distinct keys, not what any particular
key is. -/
structure IdemKey where
  value : String
deriving BEq, DecidableEq, Inhabited

/-- The `index`-th distinct key, counting from zero.

A function rather than a stream the caller walks, because a total function has
no empty case to answer for: whoever asks for key `n` gets key `n`, and there is
no unreachable branch where an adversary could crash on its own bookkeeping and
report it as the product's.

Deterministic so that a counterexample replays: the same trace mints the same
keys in the same order. -/
def idemKey (index : Nat) : IdemKey :=
  let digits := String.ofList (Nat.toDigits 16 (index + 1))
  ⟨"idem1-" ++ "".pushn '0' (32 - digits.length) ++ digits⟩

/-- Which building template a new building is laid out from.

Two values because the product has two. `confidential` is in the type and not
yet in the model's generator: a building that may not construct outward tools is
a second world, and modelling it without driving it would be a claim this
adversary has not tested. -/
inductive Template where
  | minimal
  | confidential
deriving BEq, DecidableEq, Inhabited

def Template.name : Template → String
  | .minimal => "minimal"
  | .confidential => "confidential"

instance : ToString Template where
  toString t := t.name

/-- What a halt or a release applies to. -/
inductive Scope where
  | city
  | building (addr : String)
deriving BEq, DecidableEq, Inhabited

instance : ToString Scope where
  toString
    | .city => "City"
    | .building addr => s!"Building {addr}"

/-- One thing to ask the city to do. -/
inductive Verb where
  | cityView
  | endpointView
  | createBuilding (addr : String) (template : Template) (idem : IdemKey)
  | dispatch (addr : String) (session : String) (idem : IdemKey)
  | halt (scope : Scope) (idem : IdemKey)
  | release (scope : Scope) (idem : IdemKey)
  | takeover (run : String) (idem : IdemKey)
  /-- A frame this adversary writes by hand, for the cases where the point is
  that the door refuses to encode it at all. -/
  | verbatim (raw : String)

/-- What the door said.

`quiet` is a third outcome and deliberately not a synonym for acceptance. The
door prints what arrives before the city has been silent for a window, so a
command still being served looks exactly like a command that produced nothing.
Collapsing the two is how a refused action gets read as a successful one;
adversary-SPEC section 4 records the measurement that found this. -/
inductive Answer where
  | accepted (frames : List Frame)
  | denied (complaint : Complaint)
  | quiet

instance : ToString Answer where
  toString
    | .accepted frames => s!"Accepted {frames.map toString}"
    | .denied complaint => s!"Denied {complaint}"
    | .quiet => "Quiet"

/-- How long the door waits for the city to go silent.

The client returns only after this much silence, so the window is paid in full
by every single action — the first draft set it above the product's longest
synchronous path (`PROBE_TIMEOUT_MS`, 15 s) and thereby charged thirty seconds
for a query that answers in one millisecond.

What makes a short window honest is that no verb this adversary drives touches a
slow path: the model never attaches an endpoint, and every command it sends is
answered off local disk over loopback. `quiet` therefore means "this city did
not answer promptly", and the slow path's own hazard is asserted by
`Sprawling.Model` having no action that walks it rather than by waiting for one.

Measured rather than guessed: every command the model sends answered inside one
millisecond on the machine class this was written on, so 250 ms is three orders
of margin and still two hundredths of the first draft's cost. -/
def quietMillis : Nat := 250

/-- The JSON one verb travels as.

Encoded through `Lean.Json` rather than by pasting strings together: a frame
this adversary built by hand and got subtly wrong would be reported as a refusal
by the city, and a refusal that came from a typo here is indistinguishable from
one the product owed. -/
def Verb.frame : Verb → String
  | .cityView => query "city_view"
  | .endpointView => query "endpoint_view"
  | .createBuilding addr template idem =>
    command "create_building"
      [("addr", .str addr), ("template", .str template.name), ("idem", .str idem.value)]
  | .dispatch addr session idem =>
    command "dispatch"
      [ ("addr", .str addr)
      , ("task", .str "say something")
      , ("goal", .str "an answer")
      , ("mode", .str "build")
      , ("idem", .str idem.value)
      , ("session", .str session)
      , ("effort", .null) ]
  | .halt scope idem => command "halt" [("scope", scopeValue scope), ("idem", .str idem.value)]
  | .release scope idem => command "release" [("scope", scopeValue scope), ("idem", .str idem.value)]
  | .takeover run idem => command "takeover" [("run", .str run), ("idem", .str idem.value)]
  | .verbatim raw => raw
where
  query name := (Json.mkObj [("query", .str name)]).compress
  command name fields := (Json.mkObj [("command", Json.mkObj [(name, Json.mkObj fields)])]).compress
  /-- A scope is a bare word or a one-field object, which is how serde renders
  an enum whose variants differ in whether they carry anything. -/
  scopeValue : Scope → Json
    | .city => .str "city"
    | .building addr => Json.mkObj [("building", .str addr)]

instance : ToString Verb where
  toString v := v.frame

/-- What running the program produced. -/
private structure Said where
  exitCode : UInt32
  out : String
  err : String

/-- Runs the program and takes both streams.

`IO.Process.output` drains both pipes before it waits, which is the deadlock a
hand-written version has to remember: a pipe left unread keeps the child alive,
so waiting first would hang the pair on any output larger than one buffer. -/
private def capture (binary : System.FilePath) (arguments : Array String) : IO Said := do
  let out ← IO.Process.output
    { cmd := binary.toString, args := arguments, stdin := .null }
  return { exitCode := out.exitCode, out := out.stdout, err := out.stderr }

/-- Finds the binary, preferring what the caller was told to use.

The justfile builds it and passes the path in `SPRAWLING_BIN`, so there is one
authority for where it is; the fallbacks exist only so that a person poking at
`lake env lean` is not stopped by an environment variable.

Being told a path and not finding one there is a failure, not a skip. Nothing to
drive is a fact about a machine, and skipping is the right answer to it; but a
caller that named a binary has already built it, so an empty answer there means
the name was mangled on the way in — and reporting that as a green run would
report a suite that tested nothing as a suite that passed. -/
def discover : IO (Option Door) := do
  match ← IO.getEnv "SPRAWLING_BIN" with
  | some path =>
    match ← pick [path] with
    | some door => return some door
    | none =>
      throw <| IO.userError <|
        s!"SPRAWLING_BIN names no file: {path}\n" ++
        "  On Windows this is usually a POSIX path a shell expanded; " ++
        "pass one this program can open."
  | none => pick (["debug", "release"].flatMap flavours)
where
  flavours profile :=
    [ s!"../target/{profile}/sprawling", s!"../target/{profile}/sprawling.exe" ]
  pick : List String → IO (Option Door)
    | [] => return none
    | candidate :: rest => do
      if ← System.FilePath.pathExists candidate then
        return some ⟨candidate⟩
      else
        pick rest

/-- Raises a city in a directory that does not have one. -/
def Door.raise (door : Door) (city : System.FilePath) : IO Unit := do
  let said ← capture door.binary #["init", city.toString]
  if said.exitCode != 0 then
    throw <| IO.userError s!"a city could not be raised (exit {said.exitCode}): {said.err}"

/-- A stream being emptied for as long as the city lives, with its tail kept.

A served city narrates on both of its streams, and an unread pipe stops being a
pipe once the operating system's buffer fills: the city blocks inside `write`
and answers nothing further. That failure arrives as a city which served for a
while and then went silent, which reads exactly like a product defect and is
not one.

The tail rather than the whole: diagnostics are wanted only to explain a
failure, and a city that ran for a thousand actions would otherwise be held in
memory in full. -/
private partial def siphon (stream : IO.FS.Handle) (kept : IO.Ref ByteArray) : IO Unit := do
  let chunk ← stream.read 4096
  if chunk.isEmpty then
    return ()
  kept.modify fun held =>
    let joined := held ++ chunk
    joined.extract (joined.size - min joined.size 8000) joined.size
  siphon stream kept

/-- Keeping a tail can cut a character in half, so the first bytes are dropped
until what is left decodes. At most three are ever dropped, because that is the
longest a UTF-8 sequence's continuation can be. -/
private def decodeTail (bytes : ByteArray) : String :=
  ([0, 1, 2, 3].findSome? fun dropped =>
    String.fromUTF8? (bytes.extract dropped bytes.size)).getD ""

private def drained (stream : IO.FS.Handle) : IO (IO String) := do
  let kept ← IO.mkRef ByteArray.empty
  let _ ← IO.asTask (siphon stream kept) .dedicated
  return do pure (decodeTail (← kept.get))

/-- The pipes a served city is given. -/
def servedStdio : IO.Process.StdioConfig :=
  { stdin := .null, stdout := .piped, stderr := .piped }

/-- A city that is being served, and the tail of what it complained about. -/
structure Serving where
  child : IO.Process.Child servedStdio
  said : IO String

/-- Starts serving a city, and hands back the process still running.

The caller owns its death. `--no-console` matters: without it the process reads
stdin, and a test harness has no keyboard to give it. -/
def Door.serve (door : Door) (city : System.FilePath) (port : Port) : IO Serving := do
  let child ← IO.Process.spawn
    { cmd := door.binary.toString
    , args := #["serve", city.toString, s!"127.0.0.1:{port}", "--no-console"]
    , stdin := servedStdio.stdin
    , stdout := servedStdio.stdout
    , stderr := servedStdio.stderr }
  let _ ← drained child.stdout
  let said ← drained child.stderr
  return { child, said }

/-- Stops a served city and waits for it to be gone.

Waiting matters on Windows, where a terminated process holds its port for as
long as it is unreaped, and the next ground would then find it busy. -/
def Serving.hangUp (serving : Serving) : IO Unit := do
  try serving.child.kill catch _ => pure ()
  let _ ← serving.child.wait
  return ()

/-- The refusal the client made on its own behalf, printed as text.

Two lines, shaped `E_CODE: cannot ACTION on SUBJECT` and `recovery: ...`. Read
positionally rather than by a regular expression, because what is wanted is the
code and the fact that a way forward was offered, and inventing a parser for the
sentence would make this adversary depend on its wording. -/
private def localRefusal (said : String) : Option Complaint :=
  match said.splitOn "\n" with
  | [] => none
  | headline :: rest =>
    match headline.splitOn ":" with
    | code :: remainder :: more =>
      if code.startsWith "E_" then
        some { code := ⟨code.trimAscii.toString⟩
             , action := ((remainder :: more).intersperse ":" |> String.join).trimAscii.toString
             , subject := ""
             , recovery := recoveryIn rest
             , retriable := false }
      else none
    | _ => none
where
  recoveryIn lines :=
    match lines.find? (·.startsWith "recovery:") with
    | some line => (line.drop 9).trimAscii.toString
    | none => ""

/-- Sorts what came back into the three things it can mean.

A welcome on its own is silence: the handshake proves the city is there and says
nothing about the command that followed it. So is a log line, for the same
reason and one more — a city narrates its refusals on that channel as well as
answering them on this one, so counting narration as an answer would report a
command the city never took as one it took.

There are two refusal channels and they mean different things. A `refusal` frame
on stdout is the city's judgement. A plain-text `AxError` on stderr is the
client's own: the frame was rejected before a socket was opened, which is how
`put_secret` is refused — its wire carrier is uninhabited, so the value cannot
be decoded at all. Both are refusals to the caller, and collapsing them would
lose the fact that one never reached the city. -/
private def interpret (local' : Option Complaint) (frames : List Frame) : Answer :=
  match refusalOf frames with
  | some complaint => .denied complaint
  | none =>
    match local' with
    | some complaint => .denied complaint
    | none =>
      match frames.filter (fun frame =>
          match frame with
          | .welcomed _ => false
          | .logged _ _ => false
          | _ => true) with
      | [] => .quiet
      | answered => .accepted answered

/-- Asks one question of one city, written out by hand.

A refusal is an answer. A line that will not parse is not: it means the wire has
changed shape, so this throws rather than reporting a green test against a
program it can no longer read. -/
def Door.askRaw (door : Door) (port : Port) (frame : String) : IO Answer := do
  let said ← capture door.binary
    #["call", frame, "--at", s!"127.0.0.1:{port}", "--quiet-ms", toString quietMillis]
  -- Usage errors are this adversary's own mistake, never a fact about the city.
  if said.exitCode == 2 then
    throw <| IO.userError s!"the door refused the invocation: {said.err}"
  let lines := said.out.splitOn "\n" |>.filter (fun line => !line.trimAscii.toString.isEmpty)
  let frames ← lines.mapM fun line =>
    match decodeFrame line with
    | .ok parsed => pure parsed
    | .error reason =>
      throw <| IO.userError <|
        s!"the door answered in a shape this adversary cannot read: {reason}\n" ++
        s!"  asked: {frame}\n" ++
        s!"  said:  {line.take 400}"
  return interpret (localRefusal said.err) frames

/-- Asks one question of one city. -/
def Door.ask (door : Door) (port : Port) (verb : Verb) : IO Answer :=
  door.askRaw port verb.frame

/-- Verifies a chain offline, the way `just replay` does.

`Except.error` is the refusal's first line; `Except.ok` is how far the chain
ran. Strictly read-only, which is what makes it safe to point at a ledger a city
is still writing. -/
def Door.verify (door : Door) (ledger : System.FilePath) : IO (Except String Nat) := do
  let said ← capture door.binary #["replay", ledger.toString]
  if said.exitCode == 0 then
    -- "chain verified: 4 line(s), tail seq 3"
    return .ok (tailSeq said.out)
  else
    return .error (firstLine said.err)
where
  firstLine text :=
    match text.splitOn "\n" with
    | line :: _ => line.trimAscii.toString
    | [] => "the door refused without saying why"
  tailSeq text :=
    match (text.trimAscii.toString.splitOn " ").reverse with
    | final :: _ => (final.trimAscii.toString.toNat?).getD 0
    | [] => 0

end Sprawling
