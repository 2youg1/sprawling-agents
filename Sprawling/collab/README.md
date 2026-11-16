# sprawling-collab

The mechanics that let several residents work in one building without stepping on each other. A signal goes from one resident to another and a repeated delivery does not repeat its effect; a synchronous `send` stops at a safe point and always ends; a delegated child reports back to the room that sent it; a workshop runs the nodes of a split creation in order and gathers their results; a resident claims, closes or splits a node of `Roadmap.md`; and a pull request is never verified by the resident who wrote it.

What it deliberately does not do: no I/O, and no clock of its own — the time a wait reads arrives as a parameter. Separate worktrees and disk limits are `storage::worktree`; the person's own verbs (`Steer`, `Cancel`, `Halt`, `Release`) arrive through `sprawling-wire`; a stall is `runtime::watchdog`'s; whether two goals clash is `kernel::goal`'s; the merge itself is `storage::worktree`'s.

## The seven tools live here

`signal`, `delegate`, `workshop`, `plan`, `goal`, `pr` and `archive` are the tools a resident calls for this work, and they are declared in this crate beside the mechanisms they expose. A tool is an adapter of the `kernel::tool` seam, and an adapter has to be able to name the mechanism it exposes. The dependency law in [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 lets `runtime` name only `kernel`, `storage`, `gateway`, `desktop_ffi` and `child`, so `sprawling-runtime` can never name this crate and the collaboration tools cannot be built into `runtime::tools`. Declaring them in the assembly root instead would put the decisions into the file that already holds the most of them, where `citysim` could not reach them.

## How it is shaped

Each module instantiates one of the seven shapes in [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9.

| Module | Shape | The one thing it holds |
|---|---|---|
| `inbox` | decision | At-least-once delivery, deduplicated by the signal's own id before anything that cannot be replayed; the lane is derived from the signal, never chosen by a caller. |
| `steer` | decision | Two entrances and one landing: the person's `Steer` and a resident's `Letter`, rendered in an envelope its body cannot close (`collab D16`). |
| `signal_desk` | adapter | The run's side of the traffic: the room's queue on loan, the post every signal line goes through, and the signals the run holds. |
| `signal_tool` | adapter | The face the inbox shows a model, `send` with or without `wait`, and `pull`, routed onto the run's desk. |
| `reply_wait` | decision | The waiting send: where a run stops, what ends the wait, and why two runs waiting on each other need no deadlock detection (`collab D9`). |
| `workshop` | decision | One creation split into nodes, the deterministic order they run in, and the three graphs construction refuses. |
| `workshop_tool` | adapter | Three verbs — `lay_out`, `question`, `judge` — and nothing that starts a run or decides who may. |
| `fanin` | decision | Where branches rejoin: an unverified result is a claim and claims do not join. |
| `pr` | typestate | `Open` then `Verified`, so "verified before merged" is a type rather than a rule. |
| `pr_tool` | adapter | `open`, `list`, `check`, and the refusal of a verification whose caller is the implementer. |
| `arbiter` | decision | Who settles two goals that want the same resource, and how far up it goes (`collab D1`). |
| `goal_tool` | adapter | Registering a goal against the whole register, and refusing one that clashed instead of registering it. |
| `delegate_tool` | adapter | One level down, and the desk that remembers what was asked so the assembly layer can start it at the call (`collab D7`, `collab D18`). |
| `handback` | decision | What a run is told about work it handed down, and who may say it finished. |
| `claim_tool` | adapter | The face `Roadmap.md` shows a model: seven actions on one catalog line, `list`, `claim`, `finish`, `block`, `release`, `split` and `add`. |
| `claim_effect` | value | What a claim left behind, and `still_true`, which asks whether the file still agrees with it. |
| `archive_tool` | adapter | Four closed kinds — preference, decision, correction, fact — and recall that reads the shelf rather than a copy of it. |
| `triage` | decision | Where something from outside lands, and whether it starts work. |
| `citation` | decision | A quote checked against the version pinned for review, after whitespace is normalised. |

## When a call fails

Every refusal is a `kernel::AxError` in three parts: the action, the subject, and a recovery a model can act on. A refusal changes nothing — the desk holds no node, queues no effect and edits no text — because the decision comes first and the effect is written only when the round lands. The codes a caller meets most often:

- `E_CROSS_BUILDING_DENIED` a signal or a delegate past the sender's building, and the recovery names `signal` as the way across.
- `E_GATE_DENIED` a spawn the gate refused, and a pull request the implementer tried to verify.
- `E_INVALID_ARGS` a second graph for one run, an unknown verb, a second concurrent wait, a `split` by a run that does not hold the row, and a goal that claims no resource.
- `E_CONFIG_INVALID` a `Triage` rule built on an empty match string.
- `E_GOAL_CONFLICT` a goal that clashed, with the level that decides in the recovery.
- `E_EVIDENCE_MISSING` a verification recorded against the producer's own name.
- `E_WIRE_MISMATCH` a row that is this kind of signal and cannot be read; `Handback::from_signal` answers `None` for a row that is another kind, so "this is not a handback" and "this build cannot read it" stay different answers.
- `E_TOOL_UNAVAILABLE` a pull request opened by a run with no tree, which names the building whose `RULES.toml` grants one.

Three places treat a refusal as a result rather than a fault: `Handback::of` folds the verification's error into `Stopped`, `Citation::against` answers a `Reading` instead of an error, and `Triage::decide` hands an arrival nobody could place to a person.

## Constraints

- No features, and no I/O: the crate reads no file, opens no connection and starts no thread.
- Time is a parameter. A wait's deadline is the injected clock's reading plus `PATIENCE_MS` in `signal_tool`, and the order a room's queue is read in is the Ledger's `seq`, so one trace gives one result on every platform (`collab D9`).
- A send takes effect at the call: the `signal_enqueued` line is written before the signal is delivered, because a projection may only change as a consequence of a record that already exists (`collab D7`). A signal taken out of the queue is held until a recorded model answer has read it, and a run that leaves first gives it back (`collab D8`).
- A `plan` action that splits a row requires holding it; the refusal arrives at the call rather than at landing (`collab D6`). Only `hall/mayor` may `add` the first line of an empty plan (`collab D12`).
- A parent run cancelled or failed closes the graph it laid out: no new node is handed down, and the nodes already running still report back (`collab D14`).
- Of this workspace it may name `kernel` and `storage`, and nothing else — `crates/collab/Cargo.toml` takes no other edge.

## Who uses it

Three workspace packages depend on it, as `cargo tree --workspace -i sprawling-collab -e normal --depth 1` prints: `sprawling`, the assembly root; `sprawling-accounting`, whose assembly layer builds one desk per round; and `citysim`, the scenario runner in `crates/citysim`. What it may name is its line in the `depmap` block of [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3.

## The specification

[`Spec.lean`](Spec.lean) is the entry and the parts live under [`spec/`](spec/): [`Inbox.lean`](spec/Inbox.lean), [`Delivery.lean`](spec/Delivery.lean), [`Workshop.lean`](spec/Workshop.lean), [`Fanin.lean`](spec/Fanin.lean), [`Triage.lean`](spec/Triage.lean) and [`Claim.lean`](spec/Claim.lean). [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout.

- §1 the module table, §2 what is proved and what the tests hold, §3 the two questions still open: whether a fresh session runs `done_check` before an `Artifact` is minted, and what a stopped node does to the graph.
- §7 the three neighbours' work and why the collaboration tools live here, §8 the interface of every module and why it has that shape, §10 the design choices with the alternative each one beat, §11 the boundaries each module refuses.
- §12 error handling, with `collab D1` and `collab D2` recorded there, §14 every fixed value — `ARCHIVE_KINDS` and the bytes the `plan` entry's catalog line may take — and §16 the tests.

A property that spans modules is named after the module that holds it, and the others cite that part.

## Building and checking

```bash
cargo nextest run -p sprawling-collab   # module tests, the compile-fail cases and the pull request walk
lake build crates.collab.Spec           # this crate's Lean specification
cargo xtask gates spec lexicon specalign # the SPEC, its vocabulary and its anchors
```

`crates/collab/tests/ui/` holds two programs that must fail to compile — an `Artifact` has no public constructor, and an open pull request cannot name its verifier — and `crates/collab/tests/trybuild.rs` asserts that they fail. `crates/collab/tests/pr_flow.rs` opens a pull request, has it refused, and merges it from outside.
