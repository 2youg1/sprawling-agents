# sprawling-wire

The process boundary: the frames that pass between a city and its clients, and the server that answers them. `frames` defines the three kinds a client sends or receives, Command, Query and Event, with their JSON encoding and the handshake that compares the version `WIRE_V` and a schema hash, so that a page from another build refuses to connect rather than misreading. `server` is the WebSocket server and the static assets of the page, bound to loopback by default; `reception` decides whether a request is let in and keeps the pairing and session state; `auth` mints and compares the tokens; `control` is the entry for the verbs a person uses to intervene in work already running; `aggregate` puts several cities behind one interface and forwards only Queries and Events, never a Command. This crate decides nothing about the city: a Command is carried out, a Query answered and an Event produced upstream, and this crate makes a frame that should not exist impossible to spell or refused at the handshake. Rust code names the crate `wire`; `cargo -p` takes `sprawling-wire`.

It depends on `kernel` and `documents`. `accounting` and `sprawling` depend on it in the product, `citysim` and `xtask` outside it, and the fuzz targets in `tools/fuzz/` decode its frames; `cargo tree --workspace -i sprawling-wire --depth 1 -e normal` prints the current list of workspace dependents, and the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3 holds what each crate may name.

The `server` feature brings in the listener and its TCP stack. It is a default feature of the package, but the workspace dependency turns default features off, so `sprawling` and `citysim` ask for `server` by name while `accounting`, `xtask` and the fuzz targets take the frames without a socket. The `schema` feature derives the JSON Schema of the whole envelope, from which `cargo xtask wire-ts` writes `client/src/wire.ts`; the product binary is built without it.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC lists the modules, section 8 says which part holds each interface section, section 19 in `spec/Command/Kind.lean` says from which side each verb is reached and is read by the `wiring` gate, a decision is a comment that opens with `D<n>` beside what it governs, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires. `ARCHITECTURE.md` section 7 summarises the wire, and [`docs/wire.md`](../../docs/wire.md) is the reference for a script or another agent that drives a city from outside.

When `WIRE_V` moves, `cargo xtask wire-ts --write` regenerates `client/src/wire.ts`, which is never edited by hand. A change to the wire also updates `tools/adversary/src/Sprawling/Door.lean` and the renderer in `tools/adversary/src/Sprawling/Regression.lean` in the same change, as the *Tests* section of `AGENTS.md` says, because the adversary reads the wire the way a stranger does.

## Checking it

```bash
cargo nextest run -p sprawling-wire --all-features   # the tests, as `just test` runs them
cargo check -p sprawling-wire --no-default-features  # the crate without its listener, as `just features` checks it
lake build crates.wire.Spec                          # the SPEC's proofs; `just models` builds every crate's
cargo xtask gates wire-ts wiring                     # the generated client types are current, and every verb is reachable
```

`tests/wire_contract.rs` holds the contract the rest of the system relies on: the frame counts, the schema hash, the binding decision and the two shapes a remote peer must not be able to spell. `tests/trybuild.rs` shows the compiler refusing both, a credential put onto the wire and a Command relayed through the aggregate.
