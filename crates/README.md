# crates

Every Rust package of the product lives in this directory. The packages are separate crates so that the compiler enforces the dependency rule in [`ARCHITECTURE.md`](../ARCHITECTURE.md) section 3: a crate can name only the crates its line in the `depmap` block allows, and `kernel` names none. They are not separate so that code can be reused elsewhere.

## The crates

`cargo xtask docnum --write` generates the table below from three sources: `cargo metadata` gives the directory, the package name and the lib name; the `depmap` block in `ARCHITECTURE.md` gives the crates each one may depend on; and the `[family.<name>]` duties in [`architecture.toml`](../architecture.toml) give what each one owns. The `docnum` gate turns red when the table and its sources disagree, so change a source and run the command rather than editing a row. The package name is the one `cargo -p` takes, and it carries the `sprawling-` prefix because a name on crates.io is global; the lib name is the one Rust code writes in a path.

<!-- xtask:begin crate_table -->
| Directory | Package | Lib | Owns | May depend on | SPEC |
|---|---|---|---|---|---|
| `crates/accounting` | `sprawling-accounting` | `accounting` | the city's one writer, and the ports it reaches outside itself through | `agent_protocols`, `city`, `collab`, `gateway`, `kernel`, `runtime`, `storage`, `wire` | `crates/accounting/accounting-SPEC.md` |
| `crates/agent_protocols` | `sprawling-agent-protocols` | `agent_protocols` | MCP out to a server, ACP in from an editor, and ACP out to a harness | `gateway`, `kernel` | `crates/agent_protocols/agent_protocols-SPEC.md` |
| `crates/browser` | `sprawling-browser` | `browser` | a browser driven over WebDriver BiDi, and what a model may see of a page and do to it | `kernel` | `crates/browser/browser-SPEC.md` |
| `crates/city` | `sprawling-city` | `city` | space, identity, and the documents a building keeps | `kernel` | `crates/city/city-SPEC.md` |
| `crates/collab` | `sprawling-collab` | `collab` | several residents in one building, without stepping on each other | `kernel`, `storage` | `crates/collab/Spec.lean` |
| `crates/gateway` | `sprawling-gateway` | `gateway` | everything between a decision to call a model and the bytes on the wire | `kernel` | `crates/gateway/gateway-SPEC.md` |
| `crates/kernel` | `sprawling-kernel` | `kernel` | every decision in the city, and nothing that touches a disk | nothing | `crates/kernel/kernel-SPEC.md` |
| `crates/remote_access` | `sprawling-remote-access` | `remote_access` | the remote door, and nothing that depends on the route that reaches it | `kernel` | `crates/remote_access/remote_access-SPEC.md` |
| `crates/runtime` | `sprawling-runtime` | `runtime` | one run, from dispatch to freeze | `gateway`, `kernel`, `storage` | `crates/runtime/runtime-SPEC.md` |
| `crates/sprawling` | `sprawling` | `sprawling` | the assembly root: every concrete type, the one clock, the one spawn point, and the command line | `accounting`, `agent_protocols`, `browser`, `city`, `collab`, `desktop`, `gateway`, `kernel`, `runtime`, `storage`, `wire` | `crates/sprawling/sprawling-SPEC.md` |
| `crates/storage` | `sprawling-storage` | `storage` | persistence, and every view derived from it | `kernel` | `crates/storage/storage-SPEC.md` |
| `crates/wire` | `sprawling-wire` | `wire` | the process boundary | `kernel` | `crates/wire/wire-SPEC.md` |
<!-- xtask:end -->

`desktop/` is also part of the product. It is built outside the workspace, because its Windows calls need `unsafe` code that the workspace forbids, so it has no row here.

## Adding a crate

1. A person rules on the new crate's place in the `depmap` block in `ARCHITECTURE.md` section 3, because the block is what the `depmap` gate holds every edge to.
2. The crate's SPEC is written before its code. `just spec <lib>` writes the skeleton into the crate's directory.
3. Its modules and its family are registered in `architecture.toml`, where the `modmap` gate reads them.
4. The package is named `sprawling-<lib>`, listed in the root `Cargo.toml` under `members` and `default-members`, and given one row in `[workspace.dependencies]` with its path and an exact version, which the other members inherit.
5. Then the code, under the loop in [`AGENTS.md`](../AGENTS.md).

The words the crates use are defined in [`docs/glossary.md`](../docs/glossary.md).
