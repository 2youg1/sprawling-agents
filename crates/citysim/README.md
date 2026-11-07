# citysim

Scripted scenarios on a counted clock, and the benches that time the product. A scenario drives the real turn loop with a second adapter for every port it needs, so a failure under a script replays byte for byte from that script; the same package times the product's load scenarios, its cold start, its memory and its Python arm. It owns no product behaviour: a scenario asserts on what the city did, and every rule it exercises belongs to the crate that owns that rule.

The package sits outside the product graph. Its manifest writes `role = "tool"` and `publish = false`, the `depmap` block in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 lists product crates only, and nothing here enters the `sprawling` binary or a release archive.

## What it is made of

| Module | What it owns | Shape |
| --- | --- | --- |
| `executor` | The thin executor: a counted clock, one answer at each cancellation point, an idempotency key per tool call from its position and the action's bytes (`citysim D20`), and the world handed to `runtime::run::drive`. The loop, the four phases of a turn and the freeze stay in `runtime`. | adapter |
| `mem_ledger` | The `kernel::ledger` port's second adapter: whole in memory, deterministic, writing the bytes the durable adapter writes. | adapter |
| `checker` | Two invariants over a scenario's history — the chain is whole and the `seq` numbers have no gap — asked of `runtime::replay::verify_lines`, never of a second verifier (`citysim D16`). A failed check names the 1-based line in its `subject`. | decision |
| `script_model` | The `kernel::model` seam's second adapter: the script is the whole behaviour and an exhausted script concludes. Each turn is provider wire JSON read through `gateway::response_from_wire`, so the translation is exercised and only the HTTP face is left out. | adapter |
| `script_tools` | The `kernel::tool` seam's second adapter, with every failure mode a script can inject; an exhausted script is itself `E_TOOL_UNAVAILABLE`. | adapter |
| `sieving` | A scenario's `exec` result passed through `runtime::package_exec`, the door the city uses, so a replayed window is the window the product showed. | adapter |
| `wire_script` | The script a stand-in provider plays, in the same wire JSON `ScriptModel` reads; `exchange` is one request off a loopback connection, its scripted answer, and the record line. | value, adapter |
| `red_team` | The same conclusions kept with and without a verification run, and the quality each arm keeps; the arms differ only in whether `collab::Citation::against` is called (`citysim D19`). | decision |
| `suite` | A set of real tasks split into the half a change may learn from and the half it may not; a task that appears in both halves is refused at construction. | value |
| `long_turn` | One run of many steps reading a file that changes and keeps its length, and the request window of every model call. | adapter |
| `fixture_digest` | The digest a reading names the bytes it measured by; two readings are comparable only when they agree (`citysim D8`). | value |
| `ablation`, `metabolism`, `score`, `nesting` | Evaluation instruments with no scenario caller. They compile only under test, so the library carries scenarios and not instruments (`citysim D21`). | decision |

The binaries are `bench` (the load scenarios, then the Python arm), `bench_startup` (install, startup, raising a city, opening a session, and the first byte), `long_turn` (pauses so a recipe can read the process from outside), `provider` (the stand-in on a loopback port) and `serve_grouped` (a served city in a process group of its own, which the acceptance walk uses on Windows to close a city in order).

It consumes four seams from [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 4: `kernel::ledger`, `kernel::tool` and `kernel::model`, each with citysim as its second adapter, and `runtime::turn::wave`, where a plain `FnMut` answers as it admits and so runs a wave serially. It declares no seam of its own.

## When it refuses

Errors from `kernel`, `runtime::replay` and the product's public face travel through unchanged; this crate adds no code. Its own failures use existing codes in the usual three parts:

| Code | Where it comes from | Recovery it names |
| --- | --- | --- |
| `E_INVALID_ARGS` | a clock read past the top of `u64`; a bench that took no samples | shorten the scenario or lower its per-step tick; check the selected guest and the fixed script |
| `E_CONFIG_INVALID` | a script that cannot be read, and a re-read that would rewrite a run already being answered; the `subject` is the key path | write the script as sections 8-10 and 8-13 of the SPEC state; name a file this process can read |
| `E_TOOL_UNAVAILABLE` | the stand-in's socket failed; no usable CPython-WASI guest was found | send the request again; configure a real guest through `sprawling doctor`, then rerun |
| `E_STORAGE_FATAL` | the provider's record or a measurement CSV could not be written | give the path in a directory this process can write, and start again |
| `E_PATH_NOT_FOUND` | the build directory beside the measured binary, or the measured executable itself, is missing | run `just bench-startup`, which builds the binary it measures first |
| `E_EVIDENCE_MISSING` | a city to continue holds no Ledger line, or an action recorded no samples | raise the city with `init`, which writes a genesis line; an empty reading is a defect in `bench_startup` |
| `E_DIGEST_SUSPECT` | an archive's bytes do not match the digest published with them | install nothing from it and fetch the archive again |

The stand-in's refusals to the city are HTTP answers with stable codes rather than `AxError`: 400 `body_unreadable`, 409 `runs_crossed`, 410 `script_exhausted` and 410 `no_run_left`. None of those status codes is in the set the city retries. A malformed script is refused when it is parsed, before the city hears anything: an empty run, a call id written twice, and a reply that calls no tool while a later reply in its run follows.

There is no random source here and no seed (`citysim D1`), so a red scenario replays from its script. When it does not, the defect is the determinism.

## Constraints a caller acts on

- The package has no features; a caller turns none on. `kernel` arrives with `conformance`, `wire` with `server`, and `sprawling` is named for its bench binaries only.
- A scenario is single-threaded and reads a counter instead of the system clock, and it cancels only at a turn boundary (`citysim D7`), which is why a cancel written at the same boundary beats a steer. `MemLedger` never touches a disk, and no fault filesystem is built here: that one lives in `storage`.
- Instruments live here rather than in a crate of their own, because none has a production caller (`citysim D6`).
- A bench reading is registered in `crates/xtask/budgets.toml` and never gated (`citysim D17`); the measured feature set is `sprawling`'s default features (`citysim D9`); the long turn gates on the growth of the request window, with RSS only a reading (`citysim D10`); the Python family publishes nothing without a real CPython-WASI guest (`citysim D27`); the stand-in re-reads its script only when every run it holds is used up (`citysim D15`).
- `serve_grouped` is the one Windows-only piece, with `win32job` under a `cfg(windows)` dependency.
- It may name `kernel`, `storage`, `runtime`, `gateway`, `collab`, `wire`, `sprawling` and `accounting`. The `depmap` block does not list it.

## Who uses it

No package depends on it: `cargo tree --workspace -i citysim -e normal --depth 1` prints the package alone. Its callers are the recipes `just sim`, `just bench`, `just bench-startup`, `just provider` and `just mem long-turn`, and the acceptance walk in `crates/adversary`, whose recipe starts `provider` and, on Windows, `serve_grouped`.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with a part per module under [`spec/`](spec/); [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout. Section 1 maps each module to its part, section 3 states the assumptions the proofs rest on and what is still open, section 8 lists which part defines each interface, section 12 holds the codes and the table of decisions, and section 16 names the Rust tests that hold what the proofs leave open. A decision is a comment that opens with `D<n>` and is cited as `citysim D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Checking it

```bash
cargo nextest run -p citysim      # the scenarios and the instruments' tests, as `just test` runs them
just sim                          # the same tests under `cargo test --package citysim`
lake build crates.citysim.Spec    # the SPEC's proofs; `just models` builds every specification
cargo xtask gates modmap specalign  # this crate's module rows and their SPEC anchors
```

Two golden Ledgers under `crates/fixtures/` are rebuilt byte for byte by these tests: `golden-s1` by `tests/ledger_conformance.rs` from a fixed script, and `golden-p0` by `tests/interventions.rs` from a whole run with an intervention, driven through the real turn machine. To write one again after a change that is meant to alter it, run its test with `GOLDEN_WRITE=1`, and do not edit the file by hand.

`just provider <script> <record>` starts the stand-in: it binds a loopback port, prints `SPRAWLING_PROVIDER=<url>` before it answers anything, and appends every exchange to the record.

The benches are measurement, never a gate. `just bench` runs the load scenarios, the Python arm, and the instruments that live in the product crates; `just bench-startup` builds the release binary first and then prices the four actions and the first byte. The Python arm alone is `cargo run --release -p citysim --bin bench -- python [baseline|candidate]`; it takes the guest the doctor finds usable, writes its raw observations to `target/python-bench-<arm>-<pid>.csv`, and prints the path. A missing, malformed or non-CPython guest is a prerequisite error and produces no Python readings. Change evaluation, difference-in-differences and report requirements live in [`docs/performance.md`](../../docs/performance.md).
