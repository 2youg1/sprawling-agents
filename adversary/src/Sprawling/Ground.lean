-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Door

/-!
# A city that exists for one test, and the disk's power to lie in it.

A city is one directory plus one process serving it, so a throwaway city is
both — this module owns the pair and owns their death on every path out.

Reaching into the ledger directory is not going behind the product's back.
ARCHITECTURE section 6 puts the whole history in a tree on this disk, so anybody
who can read that directory can write it; taking that position is taking the one
the design already grants an adversary. Everything an agent does still goes
through `Sprawling.Door`.
-/

namespace Sprawling

/-- The cast. Three addresses are enough for every question this wire can raise:
one to work in, one to collide with, and one nobody ever raises. -/
def cast : List String := ["acme", "beta", "gamma"]

/-- One throwaway city: a directory, and the port it is served on. -/
structure Ground where
  city : System.FilePath
  port : Port
deriving Inhabited

/-- The one history, as a directory on this disk. -/
def Ground.ledger (ground : Ground) : System.FilePath :=
  ground.city / ".sprawling" / "ledger"

/-- Every port this process may serve a city on, lent out one at a time.

Sixteen, because a suite that ever ran its groups in parallel would hold a city
per running group.

One authority for who holds a port, rather than a list each ground walks on its
own, because a listening port is exclusive and two grounds racing for one do not
merely collide — they silently swap cities. The loser's city fails to bind and
exits, and the loser's own readiness probe then reaches the winner's city on that
same port, so a whole trace runs against another test's history and reports the
answer as its own. That is worse than a red run: it was observed turning a known
defect green. -/
initialize ports : IO.Ref (List Port) ←
  IO.mkRef ((List.range 16).map (fun offset => ⟨47100 + offset⟩))

/-- Takes a port nobody else in this process holds, waiting if they all are. -/
private partial def claim : IO Port := do
  let taken ← ports.modifyGet fun held =>
    match held with
    | port :: rest => (some port, rest)
    | [] => (none, [])
  match taken with
  | some port => return port
  | none =>
    IO.sleep 50
    claim

/-- Hands a port back, after the city that held it is dead and reaped. -/
private def release (port : Port) : IO Unit :=
  ports.modify (port :: ·)

/-- Whether a city that would not serve was refused the port itself. -/
private def occupied (said : String) : Bool :=
  ["address in use", "Only one usage", "10048"].any fun needle =>
    (said.splitOn needle).length > 1

/-- Polls the city through its own client until it answers, or gives up.

The probe is `city_view` rather than a socket connect: what this adversary needs
to know is that the door works, and a port that accepts TCP before the city can
answer would start a trace against a city that is not ready.

**The budget is generous on purpose, and costs nothing when it is not needed.**
A city that answers returns on its first poll, so the try count is paid only
where a city is genuinely slow to bind — a cold page cache, a freshly linked
binary, a loaded runner. A tight budget turns all three into a failure that
names the product, and the sentence it prints is whatever the city last wrote to
its error stream, which is how an unrelated warning ends up reading like a
cause.

Two things still hold the price of a real failure down. A city gets a moment to
bind before it is asked anything, because a short sleep is cheaper than a
refused connection. And a city that has already exited is never polled again,
which is the whole cost of a port something outside this process holds. -/
private def waits (door : Door) (port : Port) (serving : Serving) : Nat → IO Bool
  | 0 => return false
  | tries + 1 => do
    IO.sleep 400
    match ← serving.child.tryWait with
    -- A city that has exited will not start answering, and every further poll
    -- would be spent proving that against a port nobody holds.
    | some _ => return false
    | none =>
      match ← door.ask port .cityView with
      | .accepted _ => return true
      | _ => waits door port serving tries

/-- Runs an action against a city that is raised, served, and then destroyed.

The process is killed on every path out, including an exception: a test run that
left cities listening would make the next run's port search fail for a reason
that has nothing to do with the product. -/
def withGround (door : Door) (act : Ground → IO α) : IO α := do
  let root ← IO.FS.createTempDir
  try
    let city := root / "city"
    door.raise city
    attempt city 4
  finally
    try IO.FS.removeDirAll root catch _ => pure ()
where
  /-- One city on one port: `none` when that port was held by something outside
  this process, which is the only case worth another try. -/
  serveOnce (city : System.FilePath) (port : Port) : IO (Option α) := do
    let serving ← door.serve city port
    try
      if ← waits door port serving 25 then
        return some (← act { city, port })
      else
        -- Read only on the failing path: the diagnostics are worth the wait
        -- exactly when there is a failure to explain.
        let complained ← serving.said
        if occupied complained then
          return none
        else
          throw <| IO.userError s!"a city would not serve: {complained}"
    finally
      serving.hangUp
  attempt (city : System.FilePath) : Nat → IO α
    | 0 =>
      throw <| IO.userError
        "no port between 47100 and 47115 could serve a city; this is the machine's, not the product's"
    | left + 1 => do
      let port ← claim
      -- A failure inside the check is the check's answer and travels on
      -- unchanged; only the port is taken back on the way out, because a
      -- checker that renamed every failure would report the harness where the
      -- product was owed.
      let raised ← try serveOnce city port catch error => do release port; throw error
      match raised with
      -- The port is deliberately not handed back. Nothing inside this process
      -- held it, so something outside does, and lending it on would give every
      -- later ground the same obstacle.
      | none => attempt city left
      | some value =>
        release port
        return value

/-- Everything the ledger holds, as bytes.

Sorted, so that a property about the history does not accidentally depend on the
order a directory happened to be walked in. -/
partial def Ground.stored (ground : Ground) : IO (List (String × ByteArray)) := do
  let gathered ← walk ground.ledger
  return (gathered.toArray.qsort (fun a b => a.fst < b.fst)).toList
where
  walk (directory : System.FilePath) : IO (List (String × ByteArray)) := do
    let entries ← directory.readDir
    let nested ← entries.toList.mapM fun entry => do
      if ← entry.path.isDir then
        walk entry.path
      else
        return [(entry.fileName, ← IO.FS.readBinFile entry.path)]
    return nested.flatten

/-- Every path a city holds outside its own reserved subtree, sorted.

The reserved subtree is excluded because it is the city's own account: the
ledger and the content store grow whenever anything happens, and a property
about "what this command left behind" means the files a person would see. -/
partial def Ground.tree (ground : Ground) : IO (List String) := do
  let gathered ← walk "" ground.city
  return (gathered.toArray.qsort (· < ·)).toList
where
  walk (prefix' : String) (directory : System.FilePath) : IO (List String) := do
    let entries ← directory.readDir
    let nested ← entries.toList.mapM fun entry => do
      if entry.fileName == ".sprawling" then
        return []
      else
        let shown := if prefix'.isEmpty then entry.fileName else prefix' ++ "/" ++ entry.fileName
        if ← entry.path.isDir then
          return shown :: (← walk shown entry.path)
        else
          return [shown]
    return nested.flatten

/-- The ledger's segments, oldest first. -/
private def segments (ground : Ground) : IO (List (String × ByteArray)) := do
  return (← ground.stored).filter fun segment => segment.fst.endsWith ".jsonl"

/-- Flips one bit inside the oldest record a ledger holds, the way damage or a
hostile disk would.

Not the end: a damaged tail is a case the product handles on purpose
(`memory::jsonl` recovers it), and hitting it would test recovery while claiming
to test detection.

Not the middle of the file either, which is what this did first. A city writes
records of its own accord — it noticed it had no provider while this was
choosing a byte — so the file's midpoint landed on a chain hash one run and on a
record's kind the next, and one test produced two different refusals. The first
line is written by `init` before anything else can happen, so it is the one
position on this disk that does not depend on timing, and a test that names a
code has to hit the same byte every time. -/
def Ground.corrupt (ground : Ground) : IO Unit := do
  match ← segments ground with
  | [] => throw <| IO.userError "the city is holding no history to corrupt"
  | (name, bytes) :: _ =>
    let newline : UInt8 := 10
    let firstLine := bytes.toList.takeWhile (· != newline) |>.length
    let position := firstLine / 2
    match bytes[position]? with
    | none => throw <| IO.userError "the history is too short to corrupt"
    | some byte =>
      IO.FS.writeBinFile (ground.ledger / name) <|
        bytes.extract 0 position ++ ByteArray.mk #[byte + 1]
          ++ bytes.extract (position + 1) bytes.size

/-- Chops the end off the newest segment, the way a power cut does.

The tail deliberately, and only here. `memory::jsonl` recovers a torn last line
on purpose, so this is the one hostile action that asks whether recovery happens
rather than whether detection does. Keeping it a separate verb from `corrupt` is
what stops either from being credited with the other's evidence: a test that
tore the tail and then claimed the history was defended would be reporting a
supported path as a caught attack. -/
def Ground.tear (ground : Ground) : IO Unit := do
  match (← segments ground).reverse with
  | [] => throw <| IO.userError "the city is holding no history to tear"
  | (name, bytes) :: _ =>
    if bytes.size ≤ 20 then
      throw <| IO.userError "the history is too short to tear"
    else
      IO.FS.writeBinFile (ground.ledger / name) (bytes.extract 0 (bytes.size - 20))

/-- Writes the oldest record a second time, the way a careless copy does.

Inside the file rather than at its end, so this is detection and not the
recovery `tear` exercises. A repeated record carries a sequence number and a
previous-hash that already belong to the line above it, so a reader that accepted
it would be reading a history two different sets of facts can explain. -/
def Ground.duplicate (ground : Ground) : IO Unit := do
  match ← segments ground with
  | [] => throw <| IO.userError "the city is holding no history to repeat"
  | (name, bytes) :: _ =>
    let newline : UInt8 := 10
    let oldest := bytes.toList.takeWhile (· != newline) |>.length
    if oldest ≥ bytes.size then
      throw <| IO.userError "the history holds no finished record to repeat"
    else
      let head := bytes.extract 0 oldest
      let rest := bytes.extract oldest bytes.size
      IO.FS.writeBinFile (ground.ledger / name)
        (head ++ ByteArray.mk #[newline] ++ head ++ rest)

end Sprawling
