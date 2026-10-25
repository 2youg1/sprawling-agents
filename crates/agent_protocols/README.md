# sprawling-agent-protocols

The two protocols the city speaks with programs outside it, in three directions:

- MCP out: a resident calls the tools of an external server a building configured. The bytes travel over stdio, HTTP or SSE, every call is recorded in the Ledger, and a recorded call replays to the same answer offline.
- ACP in: an editor drives the city as an agent, and its request becomes an ordinary dispatch.
- ACP out: the city acts as the client of an ACP agent the person consented to. The five official harnesses are built-in entries of the agent catalog, and `crates/agent_protocols/catalog/registry.json` is the snapshot of the ACP registry that ships with each release.

The package uses no `async`, holds no file handle, and builds in no provider's name. Whether a call may leave the machine is decided by the egress door in `kernel::gate`, and what comes back is tainted by `kernel::taint`; `crates/agent_protocols/Spec.lean` §7 lists the neighbours that own each of these decisions.

The `conformance` feature adds child processes that answer every line with one fixed result, for the tests of packages that wire MCP servers in. Every build a person runs leaves it off.

## Who uses it

Three workspace packages depend on it: `sprawling`, the assembly root; `sprawling-accounting`, whose writer connects a building's servers through it; and `sprawling-desktop`, the desktop connector. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/agent_protocols/Spec.lean` is the entry, and its parts live under `crates/agent_protocols/spec/`: message size and reading, tool naming, transport retries, the ACP address check, and harness consent and sessions are proved there. §3 of the entry says which parts of agent login are specified but not built yet. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-agent-protocols   # the module tests beside the code
lake build crates.agent_protocols.Spec           # this package's Lean specification
```

`crates/agent_protocols/Spec.lean` §16 names which tests check which property of the model. `just test` and `just models` run the same checks for the whole workspace.
