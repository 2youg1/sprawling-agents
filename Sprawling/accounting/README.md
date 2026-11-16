# sprawling-accounting

The city's one writer, the views every page is answered from, and the four ports the writer reaches outside itself through. Rust code names the crate `accounting`; `cargo -p sprawling-accounting` asks for the package.

`worker::RunWorker` is the writer [`docs/glossary.md`](../../docs/glossary.md) calls the *accounting thread*: it alone holds the Ledger, the endpoint book, the plans, the pursuits, the governance fold and the desks, and it settles what each run left behind in the order the results arrive. The read side lives beside it: the fold every query is answered from (`views`), every run folded into one line with its parent pointers (`lineage`), and a stretch of history exported as a playback bundle (`playback`).

What it deliberately does not do: choose the model a run talks to, renew a credential, or decide how an adapter is built. A port receives an already chosen model and an already redeemed credential, and `gateway::adapter_for` builds the adapter. Reaching this machine, a network or a terminal stays in `crates/sprawling` behind a port or a `fn` pointer (`accounting D9`).

## How it is shaped

Each module instantiates one of the seven shapes in [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9. The four ports are re-exported at the crate root, so `accounting::Clock` resolves without a module path.

| Module | Shape | The one thing it holds |
|---|---|---|
| `models` | port | `ModelFactory` builds the adapter for a `Chosen` model from a redeemed credential, and decides nothing about which model that is. |
| `connectors` | port | `Connectors` reaches one MCP server a building's configuration names and hands back the tools it offers. |
| `clock` | port | `Clock` is the only read of the time the writer and its lanes make. |
| `machine` | port | `Machine` reports what this host has and installs one item; `Recipe::command` is the only constructor of a `Runnable` (`accounting D7`). |
| `worker` | adapter | `RunWorker`, and the one door a `wire::Command` enters by. |
| `effect` | value | What a run's desks left behind: `Then` is reachable only from `Landing::record`, which appends every line before it hands the change over ([`spec/Effect.lean`](spec/Effect.lean)). |
| `plan_view` | projection | Every building's plan, parsed once and re-read only when a record names that building. |
| `views` | projection | The fold every query is answered from; `Views::rebuild` folds the Ledger again to the same bytes. |
| `lineage` | projection | Every run as one line carrying its parent pointers. |
| `playback` | projection | A stretch of the Ledger as a playback bundle, checked against another or recomputed from its city. |
| `trace` | projection | Which calls a commit is the result of. |
| `prompt_chain` | decision | The conventions a run at an address is sent, each read through that building's read bound. |
| `person`, `home` | adapter | The one layer that is the person's rather than a city's, under their home directory, and the home directory itself. |
| `guide` | adapter | The first-run guide's progress in the city's reserved subtree. |
| `held_vault`, `toolkit_broker` | adapter | A resolver over the vault behind its lock, and the three facts both sides of connecting an outside application need. |
| `offered`, `roster` | adapter | The agent offers the ACP page answered, and the city's `[[agent]]` rows read with the shipped catalog as one roster. |
| `tuning`, `listed_facts` | adapter | An endpoint's tuning translated between the wire's shape and the book's, and what an attach read, written to the CAS before its line. |

Four of these are the seam declarations [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 4 lists: `models`, `clock`, `connectors` and `machine` are declared here and implemented outside. The table's fifth row for this crate is `views::snapshot::start`'s `SnapshotFold`, which a fold implements so that a snapshot can be resumed: `views::Views` is one implementation and the worker's standing fold the other.

The production adapters are wired in `crates/sprawling`. `Hands.clock` carries the wall clock from `crates/sprawling/src/assembly/production.rs`, `Hands.machine` the doctor's machine from `crates/sprawling/src/doctor/probe.rs`, and the endpoint book's model factory and the buildings' MCP links are installed by the worker's own constructor (`accounting D18`).

## When a call fails

Every refusal is a `kernel::AxError`: the action that failed, the subject, and a recovery sentence a caller can act on. This crate defines no error type and adds no code to `AxCode` (`crates/accounting/Spec.lean` §12).

- A port's refusal passes through unchanged, so what a caller reads is `gateway`'s, `agent_protocols`' or the doctor's own three parts.
- A name the requirement table does not carry is refused rather than looked for, and a printed or manual recipe is refused with `E_TOOL_UNAVAILABLE` before anything starts.
- A confidential building refuses before the port is asked: its MCP servers are never started, and a remote model is refused with `E_GATE_DENIED` before the factory is called.
- A poisoned vault lock is `E_STORAGE_FATAL`, with the recovery "restart the server; enrolled credentials are unaffected".
- A file a person owns is never replaced to be repaired: `person::put` refuses one it cannot read, and a guide progress file this build does not read is `E_CONFIG_INVALID`.
- A refused Ledger line hands over no change: `Landing::record` returns the error and no `Then`.
- `playback::export` returns no partial bundle, `playback::land` leaves no half-written export, and `playback::check` answers five items reported apart rather than one verdict.

## Constraints

- No features: the crate compiles the same code for every consumer.
- Time is a parameter. Every moment the writer reads comes through `Hands.clock`, including the one a `log_truncated` line and a `form` genesis line are stamped with (`crates/accounting/spec/Clock.lean` §8-3).
- The writer is handed this machine as one `Hands` value (`accounting D20`), made in one place, `crates/sprawling/src/assembly/production.rs`. The worker's own tests build theirs from a fixture confined to test builds, in `worker::fixture`.
- It starts two kinds of thread, both of them named in [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 10 rule 3: one lane per run in flight in `worker::pool`, and one thread per provider request handed off in `worker::errand`. The accounting thread and the view fold are started by the assembly root.
- `unsafe_code` is forbidden by the workspace lint table, and no Zig leaf sits below this crate.
- Of this workspace it may name only what the `depmap` block allows: `agent_protocols`, `city`, `collab`, `documents`, `gateway`, `kernel`, `runtime`, `storage`, `wire`.
- The interface sections of the modules that moved here from the assembly root still live in `crates/sprawling/Spec.lean`; `accounting D15` records how and when they move (`crates/accounting/Spec.lean` §3).

## Who uses it

Two workspace packages depend on it, as `cargo tree --workspace -i sprawling-accounting -e normal --depth 1` prints: `sprawling`, the assembly root, and `citysim`, the scenario runner in `crates/citysim`. What it may name is its line in the `depmap` block of [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3.

## The specification

[`Spec.lean`](Spec.lean) is the entry and the parts live under [`spec/`](spec/). [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout.

- §1 the module table and what each one answers, §2 the nine acceptance assertions with the test that pins each, §3 what is assumed and what is still open.
- §7 the ownership table: which assembly-root module the worker holds, and which one stayed outside behind a port.
- §8 the label-to-part table for the interface sections, which keep their `§8-n` labels, and §12 error handling with the table naming the part that holds each `D<n>`.
- §16 the tests that hold what the proofs leave open, and §11 the boundary each part enumerates.

Effects are proved in [`spec/Effect.lean`](spec/Effect.lean), the plan projection in [`spec/PlanView.lean`](spec/PlanView.lean), playback selection and projection in [`spec/Playback/Select.lean`](spec/Playback/Select.lean) and [`spec/Playback/Project.lean`](spec/Playback/Project.lean), and the writer's loop in [`spec/Worker/Attend.lean`](spec/Worker/Attend.lean).

## Building and checking

```bash
cargo nextest run -p sprawling-accounting              # the module tests beside the code
cargo nextest run -p sprawling -E 'binary(acceptance)' # the three acceptance assertions
lake build crates.accounting.Spec                      # this crate's Lean specification
cargo xtask gates spec lexicon specalign               # the SPEC, its vocabulary and its anchors
```

The four ports' second implementations are the scripted factory, connectors, clock and machine in `crates/sprawling/tests/model_factory.rs`, `connectors.rs`, `clock.rs` and `machine.rs`; `cargo nextest run -p sprawling` runs them, and [`Spec.lean`](Spec.lean) §2 says what each one pins.
