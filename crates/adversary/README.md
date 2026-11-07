# adversary

A black-box property checker, written in Lean, that starts the built `sprawling` binary and attacks it through the wire from outside; beside it, the acceptance world a stranger walks with a release archive. [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 1 makes the wire the whole API and supports a second client written in any language, so this directory is a third client, written to attack rather than to use.

It reimplements none of the product's rules, holds no product code, is not a Cargo package, and is not a gate. `just check` never builds or runs it, and a red run of the scheduled checker blocks nothing: a finding is fixed in Rust, where the rule lives.

## What it checks

| Unit | Module | What it asks |
| --- | --- | --- |
| Door | [`src/Sprawling/Door.lean`](src/Sprawling/Door.lean) | The only place that knows a binary exists: one `call`, stdout's lines read as `Frame`, and an exit code read beside the frames rather than instead of them. |
| Frame | [`src/Sprawling/Frame.lean`](src/Sprawling/Frame.lean) | What the wire said, as an algebraic mirror: it parses and judges nothing. |
| Ground | [`src/Sprawling/Ground.lean`](src/Sprawling/Ground.lean) | A one-off city directory, a served process, a port, and the hostile disk actions; nothing is left behind when the ground ends. |
| Model | [`src/Sprawling/Model.lean`](src/Sprawling/Model.lean) | The state model, its postconditions and the directed attack: any prefix, one halt, any suffix, and a dispatch that must still be refused. |
| Check | [`src/Sprawling/Check.lean`](src/Sprawling/Check.lean) | Drawing traces, shrinking them and the check tree; it does not know a city exists. |
| Provider | [`src/Sprawling/Provider.lean`](src/Sprawling/Provider.lean) | A world with an endpoint attached: every spelling of one URL, a registered model's output ceiling, and two lanes running at once. |
| Layer, Person | [`src/Sprawling/Layer.lean`](src/Sprawling/Layer.lean), [`src/Sprawling/Person.lean`](src/Sprawling/Person.lean) | A file someone wrote and the answer folded from it agree, in a building's layer and in the person's own. |
| Regression | [`src/Sprawling/Regression.lean`](src/Sprawling/Regression.lean) | Two worlds' counterexamples, rendered as one Rust test file. |
| Acceptance | [`src/Sprawling/Acceptance/`](src/Sprawling/Acceptance/) | The walk with a release archive, and the checklist a person works through by hand. |

The checker is three targets of the Lean package whose `lakefile.toml` sits at the repository root: the library `Sprawling` in `src/`, and the executables `adversary` and `acceptance` in `test/`. They import only `Sprawling.*` and the Lean toolchain's own libraries, and the root `lake-manifest.json` lists no packages, so a scheduled run never fails because a registry changed.

## When a check fails

An assertion that does not hold is a finding, not an error to recover from (`adversary D1`). The report gives the seed, the number of shrinks, the minimized trace and the promise that broke; passing the seed back through `SPRAWLING_SEED` reproduces the run.

Three things are thrown rather than reported, because carrying on would read a changed door as a refusal and turn a red green: the door's shape changed (unreadable JSON, an unknown frame kind), the ground would not start (a port or a process), and a usage error (a frame built from `Verb` answered with exit 2). A seed that does not parse stops with exit 2 before any city starts, because a mistyped seed is not evidence about the product.

One failed check does not stop the tree. The whole tree runs serially (`adversary D4`), because a served city holds a port, a directory and a history, and two groups at once compete for all three.

## Constraints

- No external Lean dependency (`adversary D2`): `mkStdGen` draws the traces, `Lean.Data.Json` reads the wire, `IO.Process` runs the door and `IO.FS` holds the ground. The price is that sampling, shrinking and the check tree are written here rather than imported.
- No provider is started here (`adversary D3`). `just adversary` builds `sprawling` and hands in its path as `SPRAWLING_BIN`; `just acceptance` starts `crates/citysim`'s `provider` and hands in its URL as `SPRAWLING_PROVIDER`. The justfile is the only place that knows where either is, so a run cannot be driven by a stale binary.
- A served city is closed in order: `SIGINT` on macOS and Linux, and on Windows through citysim's `serve_grouped` launcher, which sends Ctrl-Break to the group (`adversary D8`). Only the second serving of the acceptance walk is killed; a walk that falls back to terminating a city prints a `note` line saying so.
- Ports run from 47100 upward, sixteen of them, and `127.0.0.1:47199` is the address no one on this machine listens on, which is where the provider world points. A verb that can name the frame it waits for ends on that frame, with 5000 ms as its ceiling; the rest wait for the city to fall quiet, 250 ms (`adversary D9`).
- Everything it asserts is a relation between two traces or a promise the door makes to a caller; it predicts no hash, `seq`, timestamp or idempotency key.
- Its Lean files name no Rust crate and import no product specification. What a Rust module must hold is specified in its own crate's `spec/`, and this directory cites those theorems in comments only.

## Who uses it

Nothing in the workspace depends on it: it is not a Cargo package, so it appears in no `cargo metadata`, no `depmap` edge and no release archive. `just adversary` runs the checker (and prints one line and succeeds where Lean is absent); `just acceptance <archive>` runs the walk; `.github/workflows/adversary.yml` runs the whole tree every night with a new seed.

## What a person does with a finding

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:73c6cbea8b5d39808b1dbc62dcc667a3deffea27e4474760e50cbd9ee28542a8 -->

1. Rerun with the seed the failure printed, until the trace is the same twice.
2. Read the minimized trace and the promise it broke.
3. Save the minimized counterexample as a Rust regression test under `crates/sprawling/tests/`. `SPRAWLING_ACCEPT=1` only rewrites `from_adversary.rs` from the fixed `remembered` trace and the renderer's other fixed cases; it does not save a newly discovered trace. For a different counterexample, write a separate test file beside `from_adversary.rs` so regeneration preserves it. The byte comparison checks only that the fixed generated cases match the renderer.
4. Fix the defect in Rust. This directory keeps no copy of the case.
5. When the wire changes, update [`src/Sprawling/Door.lean`](src/Sprawling/Door.lean) and the renderer in [`src/Sprawling/Regression.lean`](src/Sprawling/Regression.lean) in the same change, as the *Tests* section of [`AGENTS.md`](../../AGENTS.md) requires.

<!-- /coderabbit-ai-generated:v1 -->

## Specification

The checker's own SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/). It belongs to the `Spec` library at the repository root and is proved by `just models` like any crate's; it imports nothing from the checker, and the checker imports nothing from it (`ARCHITECTURE.md` section 11, *Specifications in Lean*). Section 1 lists the units and what each delivers, section 2 the acceptance criteria including what has been shown to bite, section 4 what the checker has found and what each finding cost to fix, section 9 the walk step by step, and section 16 what the tree does and does not prove. A decision is a comment that opens with `D<n>` and is cited as `adversary D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Running it

```bash
just adversary                          # build the binary, then run every check; one line and success where Lean is absent
just adversary --select "<text>"        # only the checks whose path in the tree contains the text; --reject leaves them out
SPRAWLING_SEED=<n> just adversary       # repeat a run with the seed a failure printed
just acceptance <archive>               # the walk a stranger takes with a release archive, beside the stand-in provider
lake build crates.adversary.Spec         # the checker's SPEC; `just models` builds every specification
```

The checks run one after another, and the order is the one that fails cheapest first: the rendered file compared byte for byte, then the door's contract, then the drawn traces, then the directed attack, then the history and the hostile disk, and last the groups named after the defects they were found from. `just acceptance` needs an archive from `just package`, because that is a release build; it unpacks into `target/acceptance/`, writes the stand-in's script from the archive's `skills/`, and records the checklist a person works through by hand as `target/acceptance/checklist.md`.
