-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Ground
import Sprawling.Check

/-!
# What has to hold between what a city was told and what it will admit.

The model remembers only what a person would remember: which buildings stand,
what is halted, and whether any provider has ever been attached. It never
predicts a sequence number, a chain hash, a timestamp or an `IdemKey` — those
are rules, they already have an authority in Rust, and restating one here is how
a second authority begins.

What it does assert is which failure a caller sees. A stable code is part of the
door's contract rather than a derived value, so pinning it pins the promise the
product makes to the agent on the other side.

**The black-box boundary is drawn by the types in this file.** Everything from
`World` down to `refusal` is pure: no signature mentions `IO`, so nothing that
decides what is owed can consult the city it is judging. Only `perform` and the
two postconditions run in `Attempt`. A model that could look before it ruled
would agree with the product by construction, which is the one way this whole
directory could report green without having checked anything.
-/

namespace Sprawling

/-- What observing an action yields.

A finite tag rather than the result type itself. Indexing on `Type` would put
the existential below in `Type 1` and drag universe polymorphism through the
whole harness; indexing on this keeps `look` typed as the only action that
answers with addresses, at no cost anywhere else. -/
inductive Yield where
  | nothing
  | addresses
deriving BEq, DecidableEq, Inhabited

/-- What a yield tag stands for. -/
abbrev Yield.Observed : Yield → Type
  | .nothing => Unit
  | .addresses => List String

/-- One thing a trace can do, indexed by what observing it yields. -/
inductive Action : Yield → Type where
  /-- Lay out a building at an address. -/
  | raise (addr : String) (template : Template) : Action .nothing
  /-- Ask for work in one, under a session name. -/
  | work (addr : String) (session : String) : Action .nothing
  /-- Stop admitting work in a scope. -/
  | stop (scope : Scope) : Action .nothing
  /-- Admit it again. -/
  | resume (scope : Scope) : Action .nothing
  /-- Take the wheel from a run — a verb the wire spells and the city does not
  perform. -/
  | seize : Action .nothing
  /-- Read the city back. -/
  | look : Action .addresses

/-- An action whose yield is not yet known to the reader, which is what a trace
is a list of. -/
structure Any where
  yield : Yield
  act : Action yield

instance : ToString (Action y) where
  toString
    | .raise addr template => s!"Raise {addr} {template}"
    | .work addr session => s!"Work {addr} {session}"
    | .stop scope => s!"Stop {scope}"
    | .resume scope => s!"Resume {scope}"
    | .seize => "Seize"
    | .look => "Look"

instance : ToString Any where
  toString packed := toString packed.act

/-- A trace is a string of actions, which is the only thing this directory ever
asserts about. -/
abbrev Trace := List Any

/-- Everything a person could know after a trace.

`minted` is a counter, not a prediction: it exists so that each action carries a
key nothing else carried, which is what keeps the product's deduplication from
turning a second distinct action into a replay of the first. -/
structure World where
  buildings : List (String × Template)
  halted : List Scope
  attached : Bool
  minted : Nat
deriving Inhabited

def initialState : World :=
  { buildings := [("hall", .minimal)], halted := [], attached := false, minted := 0 }

/-- The addresses that stand, sorted. -/
def World.standingAddresses (world : World) : List String :=
  (world.buildings.map Prod.fst |>.toArray.qsort (· < ·)).toList

def World.holds (world : World) (addr : String) : Bool :=
  world.buildings.any fun entry => entry.fst == addr

/-- Whether an address names something inside a city's own reserved subtree.

`kernel::address` answers this with one predicate over the segments rather than
with a list of protected names, and this mirrors the shape of that rule without
recomputing any of its parsing. -/
def reserved (addr : String) : Bool :=
  (addr.splitOn "/").contains ".sprawling"

/-- Whether work is admitted at an address right now. -/
def standing (world : World) (addr : String) : Bool :=
  !world.halted.contains .city && !world.halted.contains (.building addr)

/-- The refusal the model says a caller must see, if any.

The order of the guards is the order the program checks in, and getting it wrong
would not weaken the property — it would make the adversary demand a different
failure than the one the caller is entitled to. Two orderings are load-bearing
and are the reason this is written as a chain rather than as a set of
independent tests:

* A halted city answers "halted", not "no model is chosen". Swapping them would
  send a person to attach a provider when what stopped their work was a halt
  they can lift — and the recovery line is the third part of what `AxError`
  promises.
* A reserved address is refused for being reserved before it is refused for
  being occupied, because nothing may occupy it in the first place.
* For work, the halt outranks the address as well. This one is written from
  measurement rather than from taste: the first draft assumed a malformed
  address is judged first, and a halted city asked for work at
  `.sprawling/books` answers `E_GATE_DENIED`. The product is consistent about
  it — the same address under no halt answers `E_INVALID_ARGS`, and
  `create_building` at a reserved address answers `E_INVALID_ARGS` even while
  the city is halted, because laying out a building is not work and the halt
  does not cover it. -/
def refusal (world : World) : Action y → Option Code
  | .raise addr _ =>
    if reserved addr then some ⟨"E_INVALID_ARGS"⟩
    else if world.holds addr then some ⟨"E_INVALID_ARGS"⟩
    else none
  | .work addr _ =>
    -- The halt is the outermost gate on work, ahead of the address itself.
    if !standing world addr then some ⟨"E_GATE_DENIED"⟩
    else if reserved addr then some ⟨"E_INVALID_ARGS"⟩
    -- Nothing has been attached, so no tag names a model, and every dispatch
    -- stops at configuration. adversary-SPEC section 3 records that no provider
    -- is ever attached.
    --
    -- Whether the building exists is **not** asked here, and that is a measured
    -- fact about the product rather than a simplification: `dispatch_in` checks
    -- the halt, writes the brief, and only then resolves the model tag, so an
    -- address nobody raised is refused for having no model. What that costs is
    -- asserted by "a refused dispatch leaves nothing on disk" rather than
    -- smuggled into this code, because the two are different claims: this one
    -- says which refusal a caller gets, that one says what the disk looks like
    -- afterwards.
    else if !world.attached then some ⟨"E_CONFIG_INVALID"⟩
    else none
  | .stop _ => none
  | .resume _ => none
  -- Not built, and answered one verb at a time rather than by a catch-all, so
  -- the promise is that this verb refuses with a code and a way forward.
  | .seize => some ⟨"E_WIRE_MISMATCH"⟩
  | .look => none

/-- Whether the model admits this action as one that succeeds.

Whether a step is expected to succeed and whether it is expected to fail are one
question, so they are one function. Two would have to partition, and two that
stopped partitioning would schedule a step as neither. -/
def owed (world : World) (act : Action y) : Bool := (refusal world act).isNone

/-- What a step the city accepted does to what a person knows. -/
def nextState (world : World) : Action y → World
  | .raise addr template =>
    minted <| if (refusal world (Action.raise addr template)).isNone then
        { world with buildings := (addr, template) :: world.buildings }
      else world
  | .work _ _ => minted world
  | .stop scope => minted { world with halted := scope :: world.halted.erase scope }
  | .resume scope => minted { world with halted := world.halted.erase scope }
  | .seize => minted world
  | .look => world
where
  minted next := { next with minted := world.minted + 1 }

/-- What a step the city refused does to what a person knows.

A refused command spent its key at the door exactly as an accepted one does, so
the counter has to move for both. Leaving the state untouched after a refusal is
wrong here for one reason: the very next command would carry a key the city has
already answered, the product's deduplication would replay that first answer,
and the refusal a caller reads would belong to the command before the one they
sent. Only `look` is exempt, because a query carries no key. -/
def failureNextState (world : World) : Action y → World
  | .look => world
  | _ => { world with minted := world.minted + 1 }

/-- What the world becomes after a step, whichever way the step went. -/
def advance (world : World) (act : Action y) : World :=
  if owed world act then nextState world act else failureNextState world act

/-- The world a trace leaves behind. -/
def World.after (world : World) : Trace → World
  | [] => world
  | packed :: rest => (advance world packed.act).after rest

/-! ## Drawing a trace

`look` is reachable in a hand-written trace and absent from this generator,
which is not an oversight. A refused dispatch leaves a directory behind
(adversary-SPEC section 4), and `city_view` scans directories, so a random trace
containing one would fail every later `look` for a cause that has nothing to do
with the step it landed on. Generating it would therefore report one defect as
many, and hide the next one behind it. The claim itself is asserted once, by
name, in the check tree. -/

private def anAddress : Gen String :=
  -- The reserved subtree is in the generator on purpose: an address a write
  -- domain may never reach is exactly the address an adversary would try, and
  -- leaving it out would make the property vacuous.
  Gen.elements "acme" (cast ++ [".sprawling", ".sprawling/books"])

private def aSession : Gen String := Gen.elements "one" ["one", "two"]

private def aScope (world : World) : Gen Scope :=
  Gen.frequency (pure .city)
    [ (2, pure .city)
    , (1, do return .building (← Gen.elements "acme" (world.buildings.map Prod.fst ++ ["acme"]))) ]

def arbitraryAction (world : World) : Gen Any :=
  Gen.frequency (pure ⟨.nothing, .seize⟩)
    [ (4, do return ⟨.nothing, .raise (← anAddress) .minimal⟩)
    , (5, do return ⟨.nothing, .work (← anAddress) (← aSession)⟩)
    , (3, do return ⟨.nothing, .stop (← aScope world)⟩)
    , (2, do return ⟨.nothing, .resume (← aScope world)⟩)
    , (1, pure ⟨.nothing, .seize⟩) ]

/-- A trace of at most `size` actions, each drawn against the world the ones
before it left. -/
def arbitraryFrom (world : World) (size : Nat) : Gen Trace := do
  let length ← Gen.chooseNat 0 size
  grow world length
where
  grow (world : World) : Nat → Gen Trace
    | 0 => pure []
    | n + 1 => do
      let packed ← arbitraryAction world
      return packed :: (← grow (advance world packed.act) n)

def arbitraryTrace (size : Nat) : Gen Trace := arbitraryFrom initialState size

/-! ## Running a trace -/

/-- What it takes to run a trace: a built binary and a city to run it against. -/
structure Kit where
  door : Door
  ground : Ground

abbrev Attempt := ReaderT Kit IO

/-- The verb one action travels as, carrying the key this step mints. -/
private def verbOf (key : IdemKey) : Action y → Verb
  | .raise addr template => .createBuilding addr template key
  | .work addr session => .dispatch addr session key
  | .stop scope => .halt scope key
  | .resume scope => .release scope key
  | .seize => .takeover "00000000-0000-7000-8000-000000000000" key
  | .look => .cityView

/-- What the frames a city sent back mean for the action that asked.

`none` is not "no buildings": it is a door that answered a question other than
the one asked, which the caller reports as a wire change rather than as a city
in a surprising state. -/
private def project : (act : Action y) → List Frame → Option y.Observed
  | .look, frames => (answerOf "city" frames).bind cityBuildings
  | .raise .., _ => some ()
  | .work .., _ => some ()
  | .stop _, _ => some ()
  | .resume _, _ => some ()
  | .seize, _ => some ()

/-- Drives one action through the door and reads what came back.

Silence is not acceptance. The door waits a window the model's own verbs never
need, so a city that said nothing has either changed shape or stopped answering,
and either is a finding rather than a step to carry on from. -/
def perform (world : World) (act : Action y) : Attempt (Except Complaint y.Observed) := do
  let kit ← read
  let answered ← kit.door.ask kit.ground.port (verbOf (idemKey world.minted) act)
  match answered with
  | .denied complaint => return .error complaint
  | .quiet =>
    throw <| IO.userError
      s!"the city said nothing at all to {act}; see adversary-SPEC section 4"
  | .accepted frames =>
    match project act frames with
    | some value => return .ok value
    | none =>
      throw <| IO.userError
        s!"the city answered a question other than the one asked: {frames.map toString}"

/-- What an accepted action still owes, if anything. -/
def postcondition (world : World) : (act : Action y) → y.Observed → Option String
  | .look, seen =>
    if seen == world.standingAddresses then none
    else some s!"saw {seen} where {world.standingAddresses} stands"
  | .raise .., _ => none
  | .work .., _ => none
  | .stop _, _ => none
  | .resume _, _ => none
  | .seize, _ => none

/-- What a refused action still owes.

Two assertions, because a refusal is a promise in three parts and a code with no
way forward keeps only one of them. -/
def postconditionOnFailure (world : World) (act : Action y) (complaint : Complaint) :
    Option String :=
  match refusal world act with
  | some code =>
    if complaint.code != code then
      some s!"refused with {complaint.code} where {code} was owed"
    else if complaint.recovery.isEmpty then
      some s!"refused with {complaint.code} and no way forward"
    else none
  | none => some s!"refused with {complaint.code} where nothing was owed"

/-- Runs a whole trace against one city, stopping at the first broken promise.

The polarity of a step is read from `refusal` rather than declared by the
caller, so a hand-written trace cannot claim that something succeeds where the
model says it must fail. -/
partial def runTrace (world : World) : Trace → Attempt Verdict
  | [] => return .held
  | packed :: rest => do
    let act := packed.act
    match ← perform world act with
    | .ok observed =>
      match refusal world act with
      | some code => return .broke s!"{act}: this was accepted, and {code} was owed"
      | none =>
        match postcondition world act observed with
        | some why => return .broke s!"{act}: {why}"
        | none => runTrace (nextState world act) rest
    | .error complaint =>
      match postconditionOnFailure world act complaint with
      | some why => return .broke s!"{act}: {why}"
      | none => runTrace (failureNextState world act) rest

/-- Any prefix, one halt, any suffix, and work that must still be refused.

Uniform random traces reach this state rarely and by accident. Naming the attack
and quantifying over what surrounds it is the difference between a fuzzer and an
adversary, and it is the reason this directory exists at all.

The suffix may contain a release, so what is halted is read back from the model
before the closing step is appended: demanding a refusal the product does not
owe would be this adversary inventing a rule. The closing step's polarity is
then decided by `refusal` like every other step's, so the assertion that it must
fail, and fail with `E_GATE_DENIED`, is the model's own and not a second one
written here. -/
def haltIsHonoured (size : Nat) : Gen Trace := do
  let opening : Trace := [⟨.nothing, .raise "acme" .minimal⟩]
  let afterOpening := initialState.after opening
  let before ← arbitraryFrom afterOpening size
  let halt : Trace := [⟨.nothing, .stop .city⟩]
  let afterHalt := (afterOpening.after before).after halt
  let after ← arbitraryFrom afterHalt size
  let ended := afterHalt.after after
  let closing : Trace :=
    if ended.holds "acme" && !standing ended "acme" then [⟨.nothing, .work "acme" "one"⟩] else []
  return opening ++ before ++ halt ++ after ++ closing

end Sprawling
