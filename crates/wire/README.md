# sprawling-wire

The process boundary. Everything that crosses between a running city and the
programs looking at it crosses here: the frames a page, a script or another agent
sends and receives, the handshake that decides whether the two ends speak the
same wire, the listener that serves them, and the one interface that shows
several cities at once.

It decides nothing about the city. A `Command` is carried out upstream, a `Query`
is evaluated upstream, an `Event` is produced upstream; this crate owns the shapes
they are written in and the judgements a boundary makes about the peer that sent
them. Keeping a frame that should not exist from being spelled or accepted is the
work: entering a credential over the network, relaying a command between cities
and putting a secret into a serialised frame each have no spelling, or are refused
at the handshake. The crate is `wire`; `cargo -p` takes `sprawling-wire`.

## How it is shaped

Each module holds one decision:

- `frames` is the envelope both sides speak: `Hello`, `Welcome`, the client and
  server frame enums, the event and delta push, the monitor's pair of frames, and
  the wire version `WIRE_V` with `schema_hash`.
- `named_frames` is the one declaration a frame family is written in, from which
  the enum, each variant's wire name and the table the schema hash is built from
  are all generated, so the hash cannot disagree with the enum.
- `command` holds everything a client may ask the city to do. It is generic over
  the carrier of a secret, and `WireCommand` fixes that carrier to an uninhabited
  type, so a frame arriving from a socket has no representation as a `PutSecret`.
  Every state-changing command owns an `IdemKey`, and no constructor omits it.
- `answer` holds one shape per view a `Query` can return, and the closed set of
  them. `Answer::Unavailable` is a real answer and carries why.
- `carried_name` validates the names this crate does not own — a provider, a
  template, a toolkit slug — for emptiness and control characters only. Which
  values are legal stays with `gateway`, `city` and the broker.
- `reception` is every judgement the boundary makes, as pure functions with
  exhaustive verdicts, answering five questions: may this address be bound and
  which face does it then present, may this peer put a credential in the vault,
  may its opening frame be accepted, what does a frame mean in the state the
  session is in, and which records must the session be told it lost. Two more
  come before a frame exists at all: whether a request may knock (`entry`, from
  `Host`, `Origin` and `Sec-Fetch-Site`) and whether it holds this door's pairing
  (`admission`).
- `server` is the listener: serve the bundle, upgrade the socket, accept an
  upload, take a credential from a caller on this machine, pair a browser, let an
  outside editor drive the city. `auth` holds the whole life of a pairing token,
  so no other module sees its plaintext.
- `control` decides which frames intervene in work already running and what an
  intervention owes. `aggregate` puts several cities behind one interface, and
  its upstream send side accepts a `Query` and has no second method.
- `preference`, `privacy`, `guide`, `slash`, `reading`, `reply` and `assets`
  carry the values a page reads, the fold of a payload both ends perform, the
  address a refusal goes to, and the bytes of the page.

Nearly all of it is values and decisions. `server` is the adapter, and `reception`
is its thick half: the shell keeps no policy, and every branch left in it is a
send, a receive, or the end of a session. This crate declares no `pub trait`; it
is not on the seam list, and a socket's handler is injected as a concrete type.

## When a call fails

Every failure is a `kernel::AxError`: the action that failed, its subject, a
stable code, and a recovery. This crate defines none of the codes; it answers
with the ones `kernel` owns.

| Code | What produced it | What the caller does |
|---|---|---|
| `E_WIRE_MISMATCH` | `WIRE_V` or the schema hash disagrees at the handshake, or a carried name could not be read from its frame | reload the page, or send a non-empty single-line value |
| `E_CONFIG_INVALID` | a listener asked to bind off loopback with no key | supply a key or bind loopback; the process refuses to start rather than to admit |
| `E_PAIRING_REFUSED` | a pairing code or open-page code that is wrong, already used, or too early | ask for a new code |
| `E_GATE_DENIED` | a request at a door that needs pairing without one, a cross-site `Origin`, or a `Host` the listener does not answer to | present this port's credential; the refusal is a 403 before the upgrade |
| `E_INVALID_ARGS` | a value one of this crate's constructors refuses: a sample beat outside its range, a body size under its floor, a tag that is not one word, an asset path that steps outside the bundle, a pairing field that will not parse | correct the value; nothing is trimmed into shape |
| `E_STORAGE_FATAL` | the listener failing to serve, or a view read failing to answer a frame | stop and report; the wire does not retry it |

Two of the judgements refuse rather than attempt. A bind off loopback without a
key refuses before a socket exists. A request that fails the entry judgement is
refused with 403 before the upgrade, so a foreign page never reaches a handler. A
frame this build cannot read *after* the handshake is answered with one counted
refusal, and the connection stays open.

## Constraints

- **Features.** `server` carries the listener and its TCP stack. It is a default
  feature of this package and switched off on the workspace dependency, so
  `sprawling-accounting` and `xtask` take the frames without a socket and
  `crates/fuzz` takes them with default features off, while `sprawling` and
  `citysim` name `server` by hand. `schema` derives the JSON Schema of the whole
  envelope, which `cargo xtask wire-ts` renders into `client/src/wire.ts`; `xtask`
  asks for it and the product binary is built without it.
- **Concurrency.** One task drives one socket, as a `tokio::select!` over the
  peer's frames and the pushes it owes, and a view read that blocks runs on the
  blocking pool awaited by that same task. Nothing is detached.
- **The clock and randomness.** The judgements in `reception` touch no clock, no
  socket and no file: a listener hands the door a clock and a seeded random
  source, which is also what makes minting and expiry testable.
- **`unsafe`.** There is none in this crate. The socket and the HTTP routes come
  through `axum` and `tokio`, and the Ed25519 signature a paired browser answers
  a session challenge with is verified by `aws-lc-rs`.
- **What it may name.** The `depmap` block in
  [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 allows `kernel` and
  `documents`, and `cargo xtask gates depmap` holds the subset. The kernel types
  on this crate's own signatures are re-exported from `lib.rs`, because a crate
  that hands out frames owes its caller the words to read them.
- **Decisions that change how a caller uses it.** `wire D1`: `WIRE_V` moves at
  most once between two pushes, on the first commit that reshapes the wire, so a
  client change and its server change land together. `wire D2`: the types the
  city's writer names sit outside the `server` feature. `wire D54`: every caller
  presents a credential, loopback included, and the entry judgement reads `Host`
  and `Origin` before anything else. `wire D55`: a session token lives while a
  socket holds it and for 60 seconds after the last one closes. `wire D56`: a
  listener reached from beyond the machine has its own `Host` and `Origin` lists
  and the same entry judgement.

## Who uses it

`cargo tree --workspace -i sprawling-wire -e normal --depth 1` prints `citysim`,
`sprawling-accounting`, `sprawling` and `xtask`: the product binary and the
simulator serve through this crate, `sprawling-accounting` names its frames in
the writer, and `xtask` reads the schema to generate the client's types.
`crates/fuzz` is outside that workspace and decodes the frames without a socket.

## Specification

[`Spec.lean`](Spec.lean) is the crate's specification, with its parts under
[`spec/`](spec/); [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11
describes that layout. It is written in Chinese with concept names in English, as
the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.
[`docs/glossary.md`](../../docs/glossary.md) defines the words, and
[`docs/wire.md`](../../docs/wire.md) is the reference for a script or another
agent that drives a city from outside.

Where to look: section 1 lists the modules, section 3 states what about the
interface is undecided, section 8 says which part holds each interface section,
section 12 holds the error handling and the table of decisions `D<n>`, and
section 16 names the tests that hold what the proofs leave open. Section 19 lives
in `spec/Command/Kind.lean`: it says from which side each verb is reached and
which class it belongs to, and the `wiring` gate reads it there.

When the wire moves, `cargo xtask wire-ts --write` rewrites `client/src/wire.ts`,
which is never edited by hand, and `crates/adversary/src/Sprawling/Door.lean` and
`crates/adversary/src/Sprawling/Regression.lean` change in the same commit,
because the adversary reads the wire the way a stranger does.

## Checking it

```bash
cargo nextest run -p sprawling-wire --all-features   # the tests, as `just test` runs them
cargo check -p sprawling-wire --no-default-features  # the frames without the listener, as `just features` checks it
lake build crates.wire.Spec                          # the proofs; `just models` builds every crate's
cargo xtask gates wire-ts wiring                     # the client types are current, every verb is reachable
```

`tests/wire_contract.rs` holds what the rest of the system may rely on: the
command and query tables against the enums, the schema hash, the binding
judgement in all four of its cells, and the two shapes that must stay
unspellable. `tests/trybuild.rs` shows the compiler refusing both of those:
putting a credential frame onto the wire and relaying a command through the
aggregate. `tests/entry.rs` is the regression for a cross-site request refused
before the upgrade; `tests/enrolment.rs`, `tests/dropping.rs` and
`tests/recording.rs` cover the posted doors.
