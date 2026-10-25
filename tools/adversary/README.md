# adversary

A black-box property checker, written in Lean, that starts the built `sprawling` binary and attacks it through the wire from outside. `ARCHITECTURE.md` section 1 makes the wire the whole API and supports a second client written in any language; this directory is a third client, written to attack rather than to use. It draws random traces of actions against a city raised for each one and thrown away after, shrinks a failing trace, and checks properties no fixed test can reach: after any prefix, one halt and any suffix, work must still be refused; after any trace, the history the city wrote is one unbroken chain, and a byte changed in it is caught. It reimplements none of the product's rules, and it is not a gate.

It is not a Cargo package and nothing in the workspace depends on it. It is three targets of the Lean package whose `lakefile.toml` sits at the repository root: the library `Sprawling` in `src/`, and the executables `adversary` and `acceptance` in `test/`. They import only `Sprawling` and the Lean toolchain's own libraries, and the root `lake-manifest.json` lists no packages, so a scheduled run never fails because a registry changed.

## Specification

The checker's own SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/). It belongs to the `Spec` library and is proved by `just models` like any crate's, and it imports nothing from the checker, which imports nothing from it in turn (`ARCHITECTURE.md` section 11, *Specifications in Lean*). Section 1 of the SPEC lists the units of the checker, section 4 records what the checker has found and what each finding cost to fix, and a decision is a comment that opens with `D<n>`, cited as `adversary D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires. What a Rust module must hold is specified in its own crate's `spec/`, never here.

## Running it

```bash
just adversary                          # build the binary, then run every check; one line and success where Lean is absent
just adversary --select "<text>"        # only the checks whose path in the tree contains the text; --reject leaves them out
SPRAWLING_SEED=<n> just adversary       # repeat a run with the seed a failure printed
just acceptance <archive>               # the walk a stranger takes with a release archive, beside the stand-in provider
lake build tools.adversary.Spec         # the checker's SPEC; `just models` builds every specification
```

The checks run one after another, because two cities would compete for a port, a directory and a history, and one failed check does not stop the rest. `.github/workflows/adversary.yml` runs the whole tree every night with a new seed; a red run there blocks nothing and is fixed in Rust. `just acceptance` needs an archive from `just package` and starts the stand-in provider from `tools/citysim`; it writes the checklist a person works through by hand to `target/acceptance/checklist.md`.

When the adversary finds a defect, a person writes the failing case as a Rust test under `crates/sprawling/tests/`, and this directory keeps no copy of it. A change to the wire updates `src/Sprawling/Door.lean` and the renderer in `src/Sprawling/Regression.lean` in the same change, as the *Tests* section of `AGENTS.md` says.
