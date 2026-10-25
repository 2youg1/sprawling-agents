# citysim

Scripted scenarios on a counted clock, and the benches that time the product. citysim is a workspace member that ships in nothing a person downloads: its manifest marks it `role = "tool"`, and it sits outside the product's crate graph (`ARCHITECTURE.md` section 3).

A scenario drives the real turn loop, `runtime::run::drive`, with a second adapter for each port it needs: an in-memory Ledger (`mem_ledger`), a model that returns a scripted sequence (`script_model`), scripted tools into which any failure can be injected (`script_tools`), and a clock that counts instead of reading the time. Nothing in a scenario draws a random number, so a failure replays byte for byte from its script; when it does not, the defect is the determinism. `checker` holds the Ledger a scenario wrote to two invariants: the chain is whole, and the `seq` numbers have no gap. The package also holds the evaluation instruments (`suite`, `score`, `red_team` and others), the two bench binaries, the long-turn instrument, and the stand-in provider, which plays a wire script on a loopback port and records every exchange.

It depends on `kernel`, `storage`, `runtime`, `gateway`, `collab`, `wire`, `accounting` and `sprawling`, the last only for its bench binaries. No package depends on it.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC maps each module to its part, a decision is a comment that opens with `D<n>` and is cited as `citysim D<n>`, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Running it

```bash
just sim                         # every scenario: the package's test suite, `cargo test --package citysim`
cargo nextest run -p citysim     # the same suite under nextest, as `just test` runs it
lake build tools.citysim.Spec    # the SPEC's proofs; `just models` builds every specification
just bench                       # the load scenarios and the product's timing instruments; measured, never gated
just bench-startup               # install, startup, raising a city and opening a session, on the release binary
just provider <script> <record>  # the stand-in provider on a loopback port; prints SPRAWLING_PROVIDER=<url> first
just mem long-turn               # the memory counters of the long turn, read at each of its pauses
```

Two golden Ledgers under `tools/fixtures/` are rebuilt byte for byte by tests: `golden-s1` by `tests/ledger_conformance.rs` from a fixed script, and `golden-p0` by `tests/interventions.rs` from a whole run, with an intervention, driven through the real turn machine. To write one again after a change that is meant to alter it, run its test with `GOLDEN_WRITE=1`, and do not edit the file by hand.
