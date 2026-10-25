# sprawling-collab

The mechanics that let several residents work in one building without stepping on each other. A signal goes from one resident to another and a repeated delivery does not repeat its effect; a synchronous `send` waits at a safe point and always ends; a delegated child reports back to the room that sent it; a workshop runs the nodes of a split task in order and gathers their results; a resident claims, closes or splits a node of `Roadmap.md`; and a pull request is never verified by the resident who wrote it. The module table in `crates/collab/Spec.lean` §1 names the rest, from triage of an outside signal to the check that a citation still points at its pinned source.

The tools a resident calls for this work — `signal`, `delegate`, `workshop`, `plan`, `goal`, `pr` and `archive` — live here as well, because `sprawling-runtime` may not name this package, and a tool must be able to name the mechanism it exposes. Separate worktrees and disk limits belong to `storage::worktree`, the person's own verbs such as Steer and Cancel arrive through `sprawling-wire`, and stalls are handled by `runtime::watchdog`. The package does no I/O and takes the time as a parameter.

## Who uses it

Three workspace packages depend on it: `sprawling`, the assembly root; `sprawling-accounting`, the city's writer; and `citysim`, the scenario runner in `tools/citysim`. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/collab/Spec.lean` is the entry, and its parts live under `crates/collab/spec/`: inbox delivery, delivery timing and reply waits, the workshop's dispatch order, fan-in and verification, triage, and claims are proved there. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-collab   # module tests, compile-fail cases, and the pull request walk
lake build crates.collab.Spec           # this package's Lean specification
```

`crates/collab/tests/ui/` holds two programs that must fail to compile, because an `Artifact` has no public constructor and an open pull request cannot name its verifier; `crates/collab/tests/trybuild.rs` asserts that they fail. `crates/collab/tests/pr_flow.rs` opens a pull request, has it refused, and merges it from outside. `just test` and `just models` run every check for the whole workspace.
