# crates

Every Rust package of the product lives in this directory. The packages are separate crates so that the compiler enforces the dependency rule in [`ARCHITECTURE.md`](../ARCHITECTURE.md) section 3: a crate can name only the crates its line in the `depmap` block allows, and `kernel` names none. They are not separate so that code can be reused elsewhere.

## The crates

`cargo xtask docnum --write` generates the table below from three sources: `cargo metadata` gives the directory, the package name and the lib name; the `depmap` block in `ARCHITECTURE.md` gives the crates each one may depend on; and the `[family.<name>]` duties in [`architecture.toml`](../architecture.toml) give what each one owns. The `docnum` gate turns red when the table and its sources disagree, so change a source and run the command rather than editing a row. The package name is the one `cargo -p` takes, and it carries the `sprawling-` prefix because a name on crates.io is global; the lib name is the one Rust code writes in a path.

<!-- xtask:begin crate_table -->
<!-- xtask:end -->

`desktop/` is also part of the product. It is built outside the workspace, because its Windows calls need `unsafe` code that the workspace forbids, so it has no row here.

## Adding a crate

1. A person rules on the new crate's place in the `depmap` block in `ARCHITECTURE.md` section 3, because the block is what the `depmap` gate holds every edge to.
2. The crate's SPEC is written before its code. `just spec <lib>` writes the skeleton into the crate's directory.
3. Its modules and its family are registered in `architecture.toml`, where the `modmap` gate reads them.
4. The package is named `sprawling-<lib>`, listed in the root `Cargo.toml` under `members` and `default-members`, and given one row in `[workspace.dependencies]` with its path and an exact version, which the other members inherit.
5. Then the code, under the loop in [`AGENTS.md`](../AGENTS.md).

The words the crates use are defined in [`docs/glossary.md`](../docs/glossary.md).
