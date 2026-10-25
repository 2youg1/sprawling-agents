# sprawling-storage

Persistence, and every view derived from it. `jsonl` is the on-disk implementation of the kernel's Ledger port: segments of one JSON line per event, written with group commits, cut back to the last whole line after a crash, and checked line by line against the chain. Around it sit the content-addressed store (`cas`), the disposable views that are rebuilt from the Ledger when they are lost (`index`, `hot`, `sessions`, `snapshot`), the git checkpoints and worktrees a run's changes are shown and reverted through (`checkpoint`, `worktree`, `changes`, `hunks`), export and restore (`bundle`), and the walk that proves the whole chain from its first line (`chain_audit`). Every file this crate touches goes through one seam, `vfs`, whose production adapter is `real_fs`. Rust code names the crate `storage`; `cargo -p` takes `sprawling-storage`.

It depends on `kernel` and `child`. `collab`, `runtime`, `accounting` and `sprawling` depend on it in the product, and `citysim` and the fuzz targets in `tools/fuzz/` outside it; `cargo tree --workspace -i sprawling-storage --depth 1 -e normal` prints the current list of workspace dependents, and the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3 holds what each crate may name.

Two features are off by default. `fault` compiles `fault_fs`, the second adapter of the `vfs` seam, which tears writes, reorders what reaches the disk and interrupts a rename, so that the tests of another crate, `accounting` among them, can cut the power under a Ledger; this crate's own tests have it without the feature. `conformance` turns on the kernel's port suites.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC lists the modules with their shapes, section 8 says which part holds each interface section, a decision is a comment that opens with `D<n>` beside what it governs, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *Ledger*, *CAS*, *projection*, *checkpoint* and *verified prefix*. What a city keeps on disk, and where, is `ARCHITECTURE.md` section 6.

## Checking it

```bash
cargo nextest run -p sprawling-storage --all-features   # the tests, as `just test` runs them
lake build crates.storage.Spec                          # the SPEC's proofs; `just models` builds every crate's
just replay <ledger-dir>                                # `chain_audit` run by the binary on one Ledger; read-only
```

`tests/derived_views.rs` throws away the index and the hot view, rebuilds each from the same Ledger and requires the same answers. The opening of a Ledger is also tested against `tools/fixtures/ledger-ahead`, a Ledger written by a newer version, which this crate must refuse rather than read.
