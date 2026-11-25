# Sprawling

Sprawling is the city host. Its crates own project and resident identity, model dispatch, permissions, process boundaries, the Ledger and the CLI. The [app crate](app/README.md) assembles them and serves [RefRain](../RefRain/README.md). [Calyx](../Calyx/README.md) owns terminal rendering and interaction, [Mei](../Mei/README.md) is a standalone harness, and [Tools](../Tools/README.md) holds development machinery.

## Run and check a local build

From the repository root, use the toolchain pins and prerequisites in the [contribution guide](../docs/CONTRIBUTING.md#0-development-tools).

```sh
just build-web
cargo run --locked -p sprawling -- up ./cities/first
cargo test --locked -p sprawling-kernel --lib
```

For release installation and first use, follow [Getting started](../docs/start/getting-started.md). For a program driving the executable, see the [CLI and wire reference](../docs/wire.md). The [architecture](../docs/ARCHITECTURE.md) defines dependency direction. Each crate's SPEC defines its interfaces and decisions.

## The packages

`cargo xtask docnum --write` generates the table below from three sources: `cargo metadata` gives the directory, the package name and the lib name; the `depmap` block in `ARCHITECTURE.md` gives the crates each one may depend on; and the `[family.<name>]` duties in [`architecture.toml`](../architecture.toml) give what each one owns. The `docnum` gate turns red when the table and its sources disagree, so change a source and run the command rather than editing a row. The package name is the one `cargo -p` takes, and it carries the `sprawling-` prefix because a name on crates.io is global; the lib name is the one Rust code writes in a path.

<!-- xtask:begin crate_table -->
| Directory | Package | Lib | Owns | May depend on | SPEC |
|---|---|---|---|---|---|
| `Calyx` | `sprawling-calyx` | `calyx` | the production terminal shell: City and Mei console backends, and the Zig leaf that turns a scene into one frame | `accounting`, `child`, `console_ffi`, `kernel`, `wire` | `Calyx/Spec.lean` |
| `Calyx/ffi` | `sprawling-console-ffi` | `console_ffi` | the production terminal shell: City and Mei console backends, and the Zig leaf that turns a scene into one frame | nothing | `Calyx/ffi/Spec.lean` |
| `Mei` | `sprawling-mei` | `mei` | a standalone harness: one Messages request, one serial loop and one persistent Python interpreter, also as an ACP agent over stdio | nothing | `Mei/Spec.lean` |
| `Sprawling/accounting` | `sprawling-accounting` | `accounting` | the city's one writer, the views every page is answered from, and the ports the writer reaches outside itself through | `agent_protocols`, `city`, `collab`, `documents`, `gateway`, `kernel`, `runtime`, `storage`, `wire` | `Sprawling/accounting/Spec.lean` |
| `Sprawling/agent_protocols` | `sprawling-agent-protocols` | `agent_protocols` | MCP out to a server, ACP in from an editor, and ACP out to a harness | `child`, `gateway`, `kernel` | `Sprawling/agent_protocols/Spec.lean` |
| `Sprawling/app` | `sprawling` | `sprawling` | the assembly root: every concrete type, the one clock, the one spawn point, and the command line | `accounting`, `agent_protocols`, `browser`, `calyx`, `child`, `city`, `collab`, `desktop`, `desktop_ffi`, `gateway`, `kernel`, `remote_access`, `runtime`, `storage`, `wire` | `Sprawling/app/Spec.lean` |
| `Sprawling/browser` | `sprawling-browser` | `browser` | a browser driven over WebDriver BiDi, and what a model may see of a page and do to it | `kernel` | `Sprawling/browser/Spec.lean` |
| `Sprawling/child` | `sprawling-child` | `child` | the one constructor of a child process, and which environment keys a child never inherits | nothing | `Sprawling/child/Spec.lean` |
| `Sprawling/city` | `sprawling-city` | `city` | space, identity, and the documents a building keeps | `kernel` | `Sprawling/city/Spec.lean` |
| `Sprawling/collab` | `sprawling-collab` | `collab` | several residents in one building, without stepping on each other | `kernel`, `storage` | `Sprawling/collab/Spec.lean` |
| `Sprawling/desktop` | `sprawling-desktop` | `desktop` | this Windows desktop, offered as an MCP server, and the one FFI seam beneath it | `agent_protocols`, `child`, `desktop_ffi`, `kernel` | `Sprawling/desktop/Spec.lean` |
| `Sprawling/desktop/ffi` | `sprawling-desktop-ffi` | `desktop_ffi` | this Windows desktop, offered as an MCP server, and the one FFI seam beneath it | `child` | `Sprawling/desktop/ffi/Spec.lean` |
| `Sprawling/documents` | `sprawling-documents` | `documents` | the rules of a document a person reads and edits through a page: its version, its characters, its windows, and what one save changes | `kernel` | `Sprawling/documents/Spec.lean` |
| `Sprawling/gateway` | `sprawling-gateway` | `gateway` | everything between a decision to call a model and the bytes on the wire | `child`, `kernel` | `Sprawling/gateway/Spec.lean` |
| `Sprawling/kernel` | `sprawling-kernel` | `kernel` | every decision in the city, and nothing that touches a disk | nothing | `Sprawling/kernel/Spec.lean` |
| `Sprawling/remote_access` | `sprawling-remote-access` | `remote_access` | the remote door, and the routes that carry its bytes and decide nothing | `child`, `kernel` | `Sprawling/remote_access/Spec.lean` |
| `Sprawling/runtime` | `sprawling-runtime` | `runtime` | one run, from dispatch to freeze | `child`, `desktop_ffi`, `gateway`, `kernel`, `storage` | `Sprawling/runtime/Spec.lean` |
| `Sprawling/storage` | `sprawling-storage` | `storage` | persistence, and every view derived from it | `child`, `kernel` | `Sprawling/storage/Spec.lean` |
| `Sprawling/wire` | `sprawling-wire` | `wire` | the process boundary | `documents`, `kernel` | `Sprawling/wire/Spec.lean` |
<!-- xtask:end -->

Two crates have a lint table of their own, the desktop's FFI seam `Sprawling/desktop/ffi` and the console's renderer `Calyx/ffi`, each the workspace's table with `unsafe_code` at `deny`, because the declaration of a crate's Zig leaf exports and each call into them relax the lint at that one item or statement; `LEAVES` in `Tools/xtask/src/guard/wall.rs` names both, and `xtask guard` refuses any other crate that stops inheriting the workspace's table.

## Adding a crate

1. A person rules on the new crate's place in the `depmap` block in `ARCHITECTURE.md` section 3, because the block is what the `depmap` gate holds every edge to.
2. The crate's SPEC is written before its code. `just spec <lib>` writes the skeleton into the crate's directory.
3. Its modules and its family are registered in `architecture.toml`, where the `modmap` gate reads them.
4. The package is named `sprawling-<lib>`, listed in the root `Cargo.toml` under `members` and `default-members`, and given one row in `[workspace.dependencies]` with its path and an exact version, which the other members inherit.
5. Then the code, under the loop in [`AGENTS.md`](../AGENTS.md).

The words the crates use are defined in [`docs/glossary.md`](../docs/glossary.md).
