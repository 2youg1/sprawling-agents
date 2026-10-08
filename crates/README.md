# crates

Every Rust package of the product lives in this directory. The packages are separate crates so that the compiler enforces the dependency rule in [`ARCHITECTURE.md`](../ARCHITECTURE.md) section 3: a crate can name only the crates its line in the `depmap` block allows, and `kernel` names none. They are not separate so that code can be reused elsewhere.

## The crates

`cargo xtask docnum --write` generates the table below from three sources: `cargo metadata` gives the directory, the package name and the lib name; the `depmap` block in `ARCHITECTURE.md` gives the crates each one may depend on; and the `[family.<name>]` duties in [`architecture.toml`](../architecture.toml) give what each one owns. The `docnum` gate turns red when the table and its sources disagree, so change a source and run the command rather than editing a row. The package name is the one `cargo -p` takes, and it carries the `sprawling-` prefix because a name on crates.io is global; the lib name is the one Rust code writes in a path.

<!-- xtask:begin crate_table -->
| Directory | Package | Lib | Owns | May depend on | SPEC |
|---|---|---|---|---|---|
| `crates/accounting` | `sprawling-accounting` | `accounting` | the city's one writer, the views every page is answered from, and the ports the writer reaches outside itself through | `agent_protocols`, `city`, `collab`, `documents`, `gateway`, `kernel`, `runtime`, `storage`, `wire` | `crates/accounting/Spec.lean` |
| `crates/agent_protocols` | `sprawling-agent-protocols` | `agent_protocols` | MCP out to a server, ACP in from an editor, and ACP out to a harness | `child`, `gateway`, `kernel` | `crates/agent_protocols/Spec.lean` |
| `crates/browser` | `sprawling-browser` | `browser` | a browser driven over WebDriver BiDi, and what a model may see of a page and do to it | `kernel` | `crates/browser/Spec.lean` |
| `crates/child` | `sprawling-child` | `child` | the one constructor of a child process, and which environment keys a child never inherits | nothing | `crates/child/Spec.lean` |
| `crates/city` | `sprawling-city` | `city` | space, identity, and the documents a building keeps | `kernel` | `crates/city/Spec.lean` |
| `crates/collab` | `sprawling-collab` | `collab` | several residents in one building, without stepping on each other | `kernel`, `storage` | `crates/collab/Spec.lean` |
| `crates/desktop` | `sprawling-desktop` | `desktop` | this Windows desktop, offered as an MCP server, and the one FFI seam beneath it | `agent_protocols`, `child`, `desktop_ffi`, `kernel` | `crates/desktop/Spec.lean` |
| `crates/desktop/ffi` | `sprawling-desktop-ffi` | `desktop_ffi` | this Windows desktop, offered as an MCP server, and the one FFI seam beneath it | `child` | `crates/desktop/ffi/Spec.lean` |
| `crates/documents` | `sprawling-documents` | `documents` | the rules of a document a person reads and edits through a page: its version, its characters, its windows, and what one save changes | `kernel` | `crates/documents/Spec.lean` |
| `crates/gateway` | `sprawling-gateway` | `gateway` | everything between a decision to call a model and the bytes on the wire | `child`, `kernel` | `crates/gateway/Spec.lean` |
| `crates/kernel` | `sprawling-kernel` | `kernel` | every decision in the city, and nothing that touches a disk | nothing | `crates/kernel/Spec.lean` |
| `crates/remote_access` | `sprawling-remote-access` | `remote_access` | the remote door, and the routes that carry its bytes and decide nothing | `child`, `kernel` | `crates/remote_access/Spec.lean` |
| `crates/runtime` | `sprawling-runtime` | `runtime` | one run, from dispatch to freeze | `child`, `desktop_ffi`, `gateway`, `kernel`, `storage` | `crates/runtime/Spec.lean` |
| `crates/sprawling` | `sprawling` | `sprawling` | the assembly root: every concrete type, the one clock, the one spawn point, and the command line | `accounting`, `agent_protocols`, `browser`, `child`, `city`, `collab`, `desktop`, `desktop_ffi`, `gateway`, `kernel`, `remote_access`, `runtime`, `storage`, `wire` | `crates/sprawling/Spec.lean` |
| `crates/storage` | `sprawling-storage` | `storage` | persistence, and every view derived from it | `child`, `kernel` | `crates/storage/Spec.lean` |
| `crates/wire` | `sprawling-wire` | `wire` | the process boundary | `documents`, `kernel` | `crates/wire/Spec.lean` |
<!-- xtask:end -->

The desktop's FFI seam, `crates/desktop/ffi`, is the one crate with a lint table of its own, the workspace's with `unsafe_code` at `deny`, because each call into its Zig leaf relaxes the lint at that one statement.

## Adding a crate

1. A person rules on the new crate's place in the `depmap` block in `ARCHITECTURE.md` section 3, because the block is what the `depmap` gate holds every edge to.
2. The crate's SPEC is written before its code. `just spec <lib>` writes the skeleton into the crate's directory.
3. Its modules and its family are registered in `architecture.toml`, where the `modmap` gate reads them.
4. The package is named `sprawling-<lib>`, listed in the root `Cargo.toml` under `members` and `default-members`, and given one row in `[workspace.dependencies]` with its path and an exact version, which the other members inherit.
5. Then the code, under the loop in [`AGENTS.md`](../AGENTS.md).

The words the crates use are defined in [`docs/glossary.md`](../docs/glossary.md).
