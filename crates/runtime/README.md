# sprawling-runtime

One run, from dispatch to freeze. A dispatch names a run; this crate carries it
from its opening lines to its closing ones and owns what happens between them:
the turn and where an interruption is heard, the frozen prefix a request is
assembled from, the tools a model is given and what their answers become on
their way into history, the Handoff a finished run leaves behind, and the two
offline readers of a finished run.

It decides nothing about the city's meaning. The address, the task, the run
policy and the identity of a run arrive as parameters, and the moment arrives as
a hook the caller supplies, so nothing here samples a clock or mints a `RunId`.
Everything a run records is written through the `&mut dyn Ledger` a caller hands
it. The crate is `runtime`; `cargo -p` takes `sprawling-runtime`.

## How it is shaped

- `turn` is the typestate of a turn: `Assembling`, `Calling`, `ToolWave`,
  `Recording`. Every transition takes the boundary snapshot as a parameter, and
  no method can observe an interruption between two of them, so a Cancel is
  heard only where the type allows it. A wave is many calls rather than one, so
  the executor is asked before each call and a halt ends the wave there.
- `run` is the driver: dispatch, then turns until an outcome, then freeze. It
  is the single authority on the order of a run's events, and `handoff_written`
  followed by `run_frozen` is the only way out.
- `prefix` assembles the frozen prefix in the four slots `City`, `Building`,
  `Resident` and `Run`, each hashed. A volatile value has no conversion into a
  `FrozenSegment`, so a moment cannot be spelled into the prefix.
- `handoff` writes the crossing point a frozen run leaves for its successor, and
  `resume` consumes one into the next run's seed. It is never a second
  authority: on conflict, fresh reads and real execution win.
- `conversation` holds the volatile half of a request. Consecutive user content
  joins the message already open, whichever door it came through.
- `tools` is the surface a model calls — `exec`, `edit`, `read`, `search`,
  `status`, `succeed` and `tool`, the last of which lists, finds, details and
  runs a tool the run does not carry. `catalog` decides what a run is offered,
  `mode` whether its work may land, and `bench` which door a call passes.
- `pipeline` fixes the shrink order and the envelope's attachments: `sieve`
  shortens a command's output by the command that produced it, `compaction` by
  the content's shape, `offload` moves bytes to the content-addressed store,
  `elision` owns the one sentence that says bytes were cut, and `rendering` and
  `redact` draw the text a model reads and scan what it said.
- `replay` verifies a Ledger without re-executing anything, and `fork` takes a
  byte-identical prefix of it into a new run. Both are pure, and neither opens a
  socket, starts a process or writes a file.
- `backlog` is the one table of work still running and the place Halt reaches
  into; its wait is counted rather than timed.
- `watchdog` answers one question after the stall verdict: what happens next.
  Disposal is graded, a corrective Steer before a freeze, and the verdict itself
  stays `kernel`'s.
- `clock` owns the one spelling of a moment, ISO 8601 in UTC to the second, and
  when a stamp is emitted; `reminder` reads the provider's own token count rather
  than estimating a window. `reserve` is the machine budget and the queue past
  it, and `digest`, `diagnostics` and `transcript` are the frozen summary, the
  write-only log and the per-run conversation file.

Most modules are decisions or values: no I/O, no clock, no global state, an
exhaustive answer rather than a flag. `turn` is the crate's one typestate.

Two seams are declared here. `Sandbox` has one method, `run(&mut self, job:
&SandboxJob)`, implemented by `WasmtimeSandbox` behind the `wasm` feature, by
`EchoSandbox` and `FaultSandbox` for tests, and by `AbsentSandbox`, which refuses
rather than answering success for a guest that never ran. `ConcurrentInvoke` is
the wave, and its second implementation is any `FnMut`; the crate consumes
`kernel::Ledger`, `kernel::Model` and `kernel::Tool`.

## When a call fails

Every failure is a `kernel::AxError`: the action that failed, its subject, a
stable code from `AxCode::as_str`, and a recovery line. The codes a caller meets
here:

| Code | What produced it | What the caller does |
|---|---|---|
| `E_INVALID_ARGS` | an argument outside what the call admits | correct it; nothing is clamped on its behalf |
| `E_LOG_VERSION_UNSUPPORTED` | a line written by a newer binary | replay the fixture on a newer build |
| `E_CAS_CORRUPT` | a broken chain, a seq gap or a non-canonical byte | check the medium; the error names the 1-based line |
| `E_VERSION_CONFLICT` | an `edit` whose file moved between the read and the write | read again; the refusal carries the current text |
| `E_GATE_DENIED`, `E_SANDBOX_DENIED`, `E_TOOL_UNAVAILABLE` | a call the bench or the sandbox refused | nothing to retry blind; the refusal becomes a `tool_result` the model reads |
| `E_TOOL_OUTCOME_UNKNOWN` | a call with no recorded outcome | the outcome is recorded as unknown and the effect is not run again |
| `E_PROVIDER_ACCOUNTS_EXHAUSTED` | a provider failure the account round disposed of | take the round's next step |
| `E_STORAGE_FATAL` | the Ledger or the content-addressed store refused a write | stop the run; no line is repaired here |

Requests this crate refuses rather than attempts: a fork point past the run's
last line is `E_INVALID_ARGS` and is never silently clamped to the end; a guest
on a build with no execution engine is refused by `AbsentSandbox`. What the
compiler refuses cannot run at all: cancelling inside a phase, skipping a phase,
forging a Handoff, converting a volatile value into a prefix segment, and
printing a credential. `tests/trybuild.rs` expects each file under `tests/ui/` to
fail to compile.

## Constraints

- **Features.** `wasm` is off in this crate and on in the product: `sprawling`
  turns it on through its default `sandbox` feature, which is `runtime/wasm`,
  because `cargo install sprawling` must produce a binary that can execute code.
  Without it, `AbsentSandbox` refuses tool execution and names the feature in its
  recovery. `conformance` turns on `kernel/conformance`, so the kernel's port
  suites run against this crate's adapters; only `--all-features` reaches it.
- **The clock and identity.** Time arrives through `run`'s `now` hook and
  `ClockReading` for a tool face; ids arrive from the caller, which is what makes
  a scripted replay byte-identical.
- **Processes.** Every child process is configured by `child::command`: off the
  city's console, in a process group of its own, with the city's secrets removed
  from its environment. Each run's job takes a weighted CPU share and a memory
  ceiling through the `desktop_ffi` seam on Windows.
- **`unsafe` and what it may name.** There is none in this crate: file identity
  comes from `same-file`, `winapi-util` and `winx`, and the Job Object from
  `win32job`. The `depmap` block in
  [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 allows `kernel`,
  `storage`, `gateway`, `desktop_ffi` and `child`; the code takes every edge but
  `gateway`, and `cargo xtask gates depmap` holds the subset.
- **Decisions that change how a caller uses it.** `runtime D36`: the lines a
  turn has written but not yet made durable belong to the run, so a turn pays one
  barrier plus one per writing call and the run pays one at its end. `runtime
  D28`: only the driver loop writes the run-policy cell, at `BeforeWave`, and a
  tool only reads it. `runtime D7`: a run's opening and closing belong to
  `Charter`, and a harness run does not borrow `RunPlan`. `runtime D111`: the
  tool table is fixed from the `[catalog]` tables and frozen at the run's start.

## Who uses it

`cargo tree --workspace -i sprawling-runtime -e normal --depth 1` prints
`citysim`, `sprawling-accounting` and `sprawling`. `citysim` drives
`runtime::run::drive` with a scripted model, scripted tools and an in-memory
Ledger, so a scenario reproduces a run without a provider;
`sprawling-accounting` drives it on the product path; `sprawling` assembles the
concrete types.

## Specification

[`Spec.lean`](Spec.lean) is the crate's specification, with its parts under
[`spec/`](spec/); [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11
describes that layout. It is written in Chinese with concept names in English, as
the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires, and
[`docs/glossary.md`](../../docs/glossary.md) defines its words, among them
*turn*, *frozen prefix*, *held lines* and *sandbox arm*.

Where to look: section 1 lists the modules, section 3 states what about the
interface is undecided, section 8 says which part holds each interface section,
section 12 holds the error handling and the table of decisions `D<n>`, and
section 16 names the tests that hold what the proofs leave open. A decision is a
comment opening with `D<n>` above what it governs.

## Checking it

```bash
cargo nextest run -p sprawling-runtime --all-features   # the tests, with the engine, as `just test` runs them
lake build crates.runtime.Spec                          # the proofs; `just models` builds every crate's
cargo xtask gates spec specalign modmap depmap          # no sorry, every anchor on disk, every edge allowed
just sim                                                # the citysim scenarios, which drive this turn loop
```

`--all-features` is what the sandbox tests need: `tests/sandbox_a10.rs` runs
hand-written WebAssembly guests through the real engine. `tests/run_driver.rs`
pins the run's three endings, `tests/replay_fork.rs` demonstrates offline replay
and a byte-identical fork over the durable ledger, and `tests/backlog.rs`,
`tests/exec_background.rs`, `tests/exec_builds.rs`, `tests/create_limit.rs` and
`tests/reminder.rs` hold the process and limit behaviour. `just bench` runs the
instrumented paths section 10 of the SPEC describes.
