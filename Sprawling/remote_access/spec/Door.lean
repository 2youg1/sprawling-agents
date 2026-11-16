-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# The remote door: who may reach a city from outside its machine, and closesAt when.

Specifies `crates/remote_access/src/door.rs`, the state a city keeps about its remote
door: whether it is open and in which epoch, the pairing codes that are still
live, the devices that were paired, and the sessions those devices hold. The
Rust code is the authority on how these properties hold; this model is the
authority on which properties must hold (crates/remote_access/Spec.lean §8-1).

The door is independent of the route that makes it reachable (a Cloudflare
named tunnel, or a command the person wrote). A route carries bytes and
nothing else, so nothing here mentions one: every decision below is taken on
the city's machine, and a route that changes the frames it carries can only
make the door unreachable. A device's first connection, the pairing
handshake that carries its code sealed, is modelled in
`crates/remote_access/spec/Handshake.lean`; this model begins where that one
hands a code to the door.

Time is a number passed in by the caller, as everywhere in the city. The door
is closed when a city starts, and opening it draws a new epoch.

The model has six properties, with one group of theorems for each:

* **a closed door admits nothing** - no code is minted, no device is paired and
  no session is opened while the door is closed;
* **closing ends every session** - after `close`, no session is valid at any
  time, whatever it held before;
* **a code is used once, in its own epoch, before it expires** - redeeming a
  code removes it, a code from an earlier epoch is refused, and so is a code
  past its expiry;
* **a revoked device holds nothing** - revoking a device removes its sessions
  and it cannot open another;
* **a session never outlives the door** - a valid session is one of the current
  epoch whose expiry is no later than the door's own;
* **a remote session never reaches a local-only verb** - whatever the device's
  authority, the verbs that widen access or reach credentials are refused
  from outside.
-/

namespace RemoteDoor

/-- An epoch: drawn fresh each time the door opens. The Rust code draws 128
random bits and refuses to reuse the last one; here freshness is a successor. -/
abbrev Epoch := Nat
/-- A point in time, in the unit the caller samples. -/
abbrev Time := Nat
/-- A device, by the id the city gave it at pairing. -/
abbrev DeviceId := Nat
/-- A pairing code, by its secret. -/
abbrev Secret := Nat
/-- A session, by its id. -/
abbrev SessionId := Nat

/-- What a paired device may do once it holds a session. -/
inductive Authority where
  | watch
  | act
  deriving DecidableEq, Repr

/-- The three kinds of verb a frame can carry, as the door sees them. -/
inductive Verb where
  /-- Asking: reading the city, a run, a file. -/
  | read
  /-- Acting on work: dispatch, steer, stop, halt, release, answering. -/
  | act
  /-- Widening access or reaching credentials: opening the door, pairing,
  attaching an endpoint, writing rules or configuration. -/
  | localOnly
  deriving DecidableEq, Repr

/-- Whether the door is open, and closesAt when. -/
inductive Phase where
  | closed
  | open (closesAt : Time)
  deriving DecidableEq, Repr

structure Code where
  secret : Secret
  epoch : Epoch
  expires : Time
  authority : Authority

structure Device where
  id : DeviceId
  authority : Authority

structure Session where
  id : SessionId
  device : DeviceId
  epoch : Epoch
  expires : Time

structure Door where
  epoch : Epoch
  phase : Phase
  codes : List Code
  devices : List Device
  sessions : List Session

/-- The door a city starts with: closed, holding the devices it paired before. -/
def Door.start (devices : List Device) (epoch : Epoch) : Door :=
  ⟨epoch, .closed, [], devices, []⟩

def Door.isOpen (d : Door) : Bool :=
  match d.phase with
  | .closed => false
  | .open _ => true

/-- The time the door closes by itself; a closed door has none. -/
def Door.closesAt? (d : Door) : Option Time :=
  match d.phase with
  | .closed => none
  | .open u => some u

/-- Opening draws a fresh epoch and carries no code and no session over. -/
def Door.openUntil (d : Door) (closesAt : Time) : Door :=
  ⟨d.epoch + 1, .open closesAt, [], d.devices, []⟩

/-- Closing keeps the paired devices and drops everything else. -/
def Door.close (d : Door) : Door :=
  ⟨d.epoch, .closed, [], d.devices, []⟩

/-- Minting a code: only while the door is open, and bound to its epoch. -/
def Door.mint (d : Door) (secret : Secret) (expires : Time) (a : Authority) :
    Option Door :=
  if d.isOpen then
    some { d with codes := ⟨secret, d.epoch, expires, a⟩ :: d.codes }
  else none

/-- Whether a code may be redeemed at `now`. -/
def Door.redeemable (d : Door) (now : Time) (c : Code) : Bool :=
  d.isOpen && c.epoch == d.epoch && decide (now < c.expires)

/-- Redeeming a code pairs the device `id` and removes every code with that
secret, so a second attempt finds nothing. -/
def Door.redeem (d : Door) (secret : Secret) (id : DeviceId) (now : Time) :
    Option Door :=
  match d.codes.find? (fun c => c.secret == secret) with
  | none => none
  | some c =>
    if d.redeemable now c then
      some { d with
        codes := d.codes.filter (fun c' => c'.secret != secret)
        devices := ⟨id, c.authority⟩ :: d.devices }
    else none

/-- Revoking a device removes it and every session it holds. -/
def Door.revoke (d : Door) (id : DeviceId) : Door :=
  { d with
    devices := d.devices.filter (fun v => v.id != id)
    sessions := d.sessions.filter (fun s => s.device != id) }

def Door.paired (d : Door) (id : DeviceId) : Bool :=
  d.devices.any (fun v => v.id == id)

/-- Admitting a paired device (its handshake already proved its key) opens a
session that ends no later than the door does. -/
def Door.admit (d : Door) (sid : SessionId) (id : DeviceId) (expires : Time) :
    Option Door :=
  match d.phase with
  | .closed => none
  | .open closesAt =>
    if d.paired id then
      some { d with sessions := ⟨sid, id, d.epoch, min expires closesAt⟩ :: d.sessions }
    else none

/-- A session is valid at `now` when the door is open in its epoch, the
session is held, its device is still paired, and it has not expired. -/
def Door.valid (d : Door) (now : Time) (s : Session) : Bool :=
  d.isOpen && s.epoch == d.epoch && d.sessions.any (fun t => t.id == s.id)
    && d.paired s.device && decide (now < s.expires)

/-- What a device's authority lets a remote session do. -/
def permits : Authority → Verb → Bool
  | _, .read => true
  | .act, .act => true
  | .watch, .act => false
  | _, .localOnly => false

/-! ## A closed door admits nothing -/

theorem closed_mints_nothing {d : Door} (h : d.phase = .closed)
    (secret : Secret) (e : Time) (a : Authority) : d.mint secret e a = none := by
  simp [Door.mint, Door.isOpen, h]

theorem closed_redeems_nothing {d : Door} (h : d.phase = .closed)
    (secret : Secret) (id : DeviceId) (now : Time) : d.redeem secret id now = none := by
  unfold Door.redeem
  split
  · rfl
  · simp [Door.redeemable, Door.isOpen, h]

theorem closed_admits_nothing {d : Door} (h : d.phase = .closed)
    (sid : SessionId) (id : DeviceId) (e : Time) : d.admit sid id e = none := by
  simp [Door.admit, h]

/-! ## Closing ends every session -/

theorem close_ends_every_session (d : Door) (now : Time) (s : Session) :
    d.close.valid now s = false := by
  simp [Door.close, Door.valid, Door.isOpen]

/-- Opening again does not bring a session back: the new door holds none. -/
theorem reopen_holds_no_session (d : Door) (closesAt now : Time) (s : Session) :
    (d.close.openUntil closesAt).valid now s = false := by
  simp [Door.close, Door.openUntil, Door.valid]

/-! ## A code is used once, in its own epoch, before it expires -/

theorem redeem_removes_the_code {d d' : Door} {secret : Secret} {id : DeviceId}
    {now : Time} (h : d.redeem secret id now = some d') :
    d'.codes.all (fun c => c.secret != secret) = true := by
  unfold Door.redeem at h
  split at h
  · contradiction
  · split at h
    · cases h
      simp [List.all_filter]
    · contradiction

theorem code_is_used_once {d d' : Door} {secret : Secret} {id id' : DeviceId}
    {now now' : Time} (h : d.redeem secret id now = some d') :
    d'.redeem secret id' now' = none := by
  have gone := redeem_removes_the_code h
  unfold Door.redeem
  split
  · rfl
  · rename_i c hc
    have mem := List.mem_of_find?_eq_some hc
    have same := List.find?_some hc
    have other := List.all_eq_true.mp gone c mem
    simp_all

/-- Codes minted before the door closed are gone, and the epoch has moved, so
none of them can pair a device after it opens again. -/
theorem stale_epoch_is_refused (d : Door) (closesAt : Time) (c : Code)
    (old : c.epoch ≤ d.epoch) (now : Time) :
    (d.close.openUntil closesAt).redeemable now c = false := by
  have fresh : (c.epoch == d.epoch + 1) = false := by
    rw [beq_eq_false_iff_ne]
    intro same
    rw [same] at old
    exact Nat.not_succ_le_self d.epoch old
  simp [Door.redeemable, Door.close, Door.openUntil, fresh]

theorem expired_code_is_refused (d : Door) (c : Code) {now : Time}
    (late : c.expires ≤ now) : d.redeemable now c = false := by
  simp [Door.redeemable, Nat.not_lt.mpr late]

/-! ## A revoked device holds nothing -/

theorem revoked_holds_no_session (d : Door) (id : DeviceId) (now : Time)
    (s : Session) (mine : s.device = id) : (d.revoke id).valid now s = false := by
  simp [Door.valid, Door.revoke, Door.paired, mine]

theorem revoked_is_not_admitted (d : Door) (id : DeviceId) (sid : SessionId)
    (e : Time) : (d.revoke id).admit sid id e = none := by
  unfold Door.admit
  split
  · rfl
  · simp [Door.paired, Door.revoke]

/-! ## A session never outlives the door -/

theorem admitted_session_ends_with_the_door {d d' : Door} {sid : SessionId}
    {id : DeviceId} {e closesAt : Time} (h : d.admit sid id e = some d')
    (opened : d.phase = .open closesAt) :
    ∀ s ∈ d'.sessions, s ∈ d.sessions ∨ (s.epoch = d.epoch ∧ s.expires ≤ closesAt) := by
  unfold Door.admit at h
  rw [opened] at h
  simp only at h
  split at h
  · cases h
    intro s hs
    simp only [List.mem_cons] at hs
    rcases hs with rfl | old
    · exact Or.inr ⟨rfl, Nat.min_le_right e closesAt⟩
    · exact Or.inl old
  · contradiction

/-! ## A remote session never reaches a local-only verb -/

theorem local_only_is_never_remote (a : Authority) : permits a .localOnly = false := by
  cases a <;> rfl

theorem watch_only_reads (v : Verb) (h : permits .watch v = true) : v = .read := by
  cases v <;> simp_all [permits]

end RemoteDoor
