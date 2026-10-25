# sprawling-kernel

Every decision in the city, and nothing that touches a disk. The kernel is a layer of pure functions: each one takes its arguments and returns a verdict, and the crate holds nothing on disk. Addresses and the reserved prefix, the event envelope and the Ledger's chain, the event kinds, the error codes and their recoveries, the Gate and its doors, plans, the policy constants and the layout of a city on disk are defined here, and every other crate builds on those definitions instead of keeping its own. Rust code names the crate `kernel`; `cargo -p` takes `sprawling-kernel`.

It depends on no other crate of the workspace, which is the first line of the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3. In the other direction, every crate of the product except `child`, `console_ffi` and `desktop_ffi` depends on it, and so do the tools `citysim` and `xtask` and the fuzz targets in `tools/fuzz/`; `cargo tree --workspace -i sprawling-kernel --depth 1 -e normal` prints the current list. The seams the outer crates implement, among them the Ledger, a tool and a model, are traits declared here; the seam table in `ARCHITECTURE.md` section 4 names each one and its adapters.

Two features are off by default. `conformance` carries the assertion suites every implementation of a port must pass, and `schema` derives the JSON Schema of every value that crosses the wire, which `cargo xtask wire-ts` reads through `sprawling-wire`; the product binary is built without either.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with one part per module under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Its section 8 says which part holds each interface section, a decision is a comment that opens with `D<n>` beside what it governs, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines the words.

## Checking it

```bash
cargo nextest run -p sprawling-kernel --all-features   # the tests, with both features on, as `just test` runs them
lake build crates.kernel.Spec                          # the SPEC's proofs; `just models` builds every crate's
just proof                                             # the kani harnesses, which need kani and a Linux or macOS host
```

`tests/trybuild.rs` compiles each file under `tests/ui/` and expects the compiler to refuse it, which is how an invalid state is shown to be unrepresentable rather than only rejected. The rest of the loop, and when each tier runs, is in `AGENTS.md`.
