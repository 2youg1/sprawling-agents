# tools

This directory holds what checks, simulates, fuzzes and attacks the product, and none of it is part of what a person downloads: nothing here is compiled into the `sprawling` binary, and nothing here is in the release archive, whose contents are the table `ARCHIVE` in `tools/xtask/src/package/contents.rs`. The `artifact` gate holds the first half, by refusing test scaffolding inside the binary.

| Directory | What it is |
|---|---|
| [`tools/xtask/`](./xtask/) | The machine gates and the build and release commands, run as `cargo xtask <command>`. `cargo xtask gates --list` prints every gate; [`xtask-SPEC.md`](./xtask/xtask-SPEC.md) says what each one judges. |
| [`tools/citysim/`](./citysim/) | Fixed scenario scripts on a counted clock, with no random source, so a failure replays byte for byte from its scenario; and the benches that time the product. |
| [`tools/fuzz/`](./fuzz/) | The cargo-fuzz targets for the parsers that read outside input: addresses, locators, a ledger's tail, wire frames, the configuration file and what an MCP server writes back. It is a workspace of its own and needs a nightly toolchain and cargo-fuzz. |
| [`tools/adversary/`](./adversary/) | A black-box checker in Lean that starts the built binary and attacks it through the wire from outside; it is never a gate. What a Rust module must hold is specified under its crate's `spec/` and proved by `just models`. The checker is two targets of the Lean package at the repository root, whose `lean-toolchain` the product binary also embeds, which is why the pin does not live under `tools/`. |
| [`tools/fixtures/`](./fixtures/) | Data whose bytes are the assertion: `address.jsonl`, the address spellings with their verdicts, read by the kernel's tests and the client's; `golden-p0` and `golden-s1`, ledgers that citysim rebuilds byte for byte; `ledger-ahead`, a ledger from a newer version that storage must refuse; `remote-handshake`, the vectors and the one-way fixture the browser's remote door must reproduce, written by the `remote_access` tests and read by them and the client's (section 8-12 of `crates/remote_access/Spec.lean`). Regenerate a golden ledger by running its citysim test with `GOLDEN_WRITE=1`, and `remote-handshake` by running the `remote_access` tests named `vector` or `fixture` the same way, not by editing either. |

The commands that run these tools are recipes in the root `justfile`; `just --list` prints them with what each one does.
