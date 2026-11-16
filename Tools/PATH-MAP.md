# F2 path map — crates/ → Sprawling/ + Tools/

Mechanical migration only. Package, lib, and online protocol names are unchanged.

| Old path | New path | Kind |
|---|---|---|
| `crates/accounting` | `Sprawling/accounting` | product |
| `crates/agent_protocols` | `Sprawling/agent_protocols` | product |
| `crates/browser` | `Sprawling/browser` | product |
| `crates/child` | `Sprawling/child` | product |
| `crates/city` | `Sprawling/city` | product |
| `crates/collab` | `Sprawling/collab` | product |
| `crates/desktop` | `Sprawling/desktop` | product |
| `crates/documents` | `Sprawling/documents` | product |
| `crates/gateway` | `Sprawling/gateway` | product |
| `crates/kernel` | `Sprawling/kernel` | product |
| `crates/remote_access` | `Sprawling/remote_access` | product |
| `crates/runtime` | `Sprawling/runtime` | product |
| `crates/sprawling` | `Sprawling/app` | product |
| `crates/storage` | `Sprawling/storage` | product |
| `crates/wire` | `Sprawling/wire` | product |
| `crates/desktop/ffi` | `Sprawling/desktop/ffi` | product |
| `crates/adversary` | `Tools/adversary` | tool |
| `crates/benchstat` | `Tools/benchstat` | tool |
| `crates/citysim` | `Tools/citysim` | tool |
| `crates/fixtures` | `Tools/fixtures` | tool |
| `crates/fuzz` | `Tools/fuzz` | tool |
| `crates/xtask` | `Tools/xtask` | tool |
| `crates/README.md` | `Sprawling/README.md (+ Tools/README.md)` | docs |
| `crates/development-tools.md` | `Tools/development-tools.md` | docs |


The mechanical mover used for this split is [`f2-migrate.py`](./f2-migrate.py).
