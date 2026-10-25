# sprawling-runtime

One run, from dispatch to freeze. Each turn of a run passes through four phases, and `turn` makes an interruption inside a phase something that does not compile, so a Cancel is heard only at the boundaries between them. Around the turn sit the frozen prefix and its four segments (`prefix`), the conversation the run carries forward (`conversation`), the tools a model is given and the catalog that lists them (`tools`, `catalog`), the envelope and the deterministic shortening of tool results (`pipeline`, `sieve`, `compaction`), the Handoff, the backlog that Halt reaches into, and `replay` and `fork`, which verify the chain offline and branch a conversation from it. `run` is the one authority on the order of events from dispatch to freeze. Rust code names the crate `runtime`; `cargo -p` takes `sprawling-runtime`.

It depends on `kernel`, `storage` and `child`, and on Windows on `desktop_ffi`, through which each run's job takes its CPU share. `accounting` and `sprawling` depend on it in the product, and `citysim` drives its turn loop a second time with a scripted model, scripted tools and an in-memory Ledger (`ARCHITECTURE.md` section 3). `cargo tree --workspace -i sprawling-runtime --depth 1 -e normal` prints the current list of dependents, and the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3 holds what each crate may name.

The `wasm` feature brings in the wasmtime engine behind the `sandbox` seam, the arm that runs Python inside wasip1. It is off here and on in the product, because the `sprawling` package turns it on through its default `sandbox` feature; `conformance` turns on the kernel's port suites.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC lists the modules, section 8 says which part holds each interface section, a decision is a comment that opens with `D<n>` beside what it governs, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *turn*, *frozen prefix*, *held lines* and *sandbox arm*.

## Checking it

```bash
cargo nextest run -p sprawling-runtime --all-features   # the tests, with the wasm engine, as `just test` runs them
lake build crates.runtime.Spec                          # the SPEC's proofs; `just models` builds every crate's
just sim                                                # the citysim scenarios, which drive this crate's turn loop
```

`tests/trybuild.rs` compiles each file under `tests/ui/` and expects the compiler to refuse it: each file is something this crate makes impossible to write, such as cancelling inside a phase, skipping one, or putting a volatile value into the frozen prefix. `tests/sandbox_a10.rs` runs hand-written WebAssembly guests through the real engine; it compiles only with the `wasm` feature, which is one reason the command above passes `--all-features`.
