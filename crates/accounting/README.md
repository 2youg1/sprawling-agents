# sprawling-accounting

The home of the accounting thread, the city's one writer. It alone holds the Ledger, the endpoint book, the plans, the pursuits, the governance fold and the desks, and it settles what each run left behind in the order the results arrive. The package also answers every question a page asks, by folding Ledger records into views, and it exports a stretch of the Ledger as a playback bundle.

The writer reaches outside itself only through ports: which model adapter a run talks to, how a configured MCP server becomes tools, what time it is, and what the host machine has installed. The production adapters behind those ports are wired in by the assembly root in `crates/sprawling`, and `tools/citysim` drives the same writer with second implementations of them, so a scenario replays a whole dispatch rather than one run. What moved into this package and what stays in `crates/sprawling` behind a port is a table in `crates/accounting/Spec.lean` §7.

## Who uses it

Two workspace packages depend on it: `sprawling`, the assembly root, and `citysim`, the scenario runner in `tools/citysim`. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/accounting/Spec.lean` is the entry, and its parts live under `crates/accounting/spec/`: the effect order, the plan view, the machine recipes, the accounting thread's loop, and playback selection and projection are proved there, and the remaining parts record interface shapes and decisions. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out. The words are defined in `docs/glossary.md`.

## Building and checking

```bash
cargo nextest run -p sprawling-accounting   # the module tests beside the code
lake build crates.accounting.Spec           # this package's Lean specification
```

`crates/accounting/Spec.lean` §16 names which tests check which property of the model. `just test` and `just models` run the same checks for the whole workspace.
