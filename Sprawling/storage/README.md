# sprawling-storage

Persistence, and every view derived from it. The Ledger is here: `jsonl` is the on-disk implementation of the kernel's `Ledger` port, one JSON line per event, written with one durability barrier per wave, cut back to the last whole line after a crash, and checked line by line against the chain. Around it sit the content-addressed store, the disposable views a query is answered from, the git checkpoints and worktrees a run's changes are shown and reverted through, export and restore, and the walk that proves the whole chain from its first line.

What it deliberately does not do: decide what an event means. The kind split, payload validation and the canonical line are the kernel's, and `jsonl` owns only `seq`, `prev` and the medium. It samples no clock, so the time a `log_truncated` line carries is a parameter of `open`. It never exposes a segment boundary, and it reads no derived view to decide anything.

## How it is shaped

Each module instantiates one of the seven shapes in [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9, and [`Spec.lean`](Spec.lean) §1 gives the shape of each one. Four groups organise the list.

**The Ledger is the one thing that is history.** `jsonl` owns the segments, `seq` and `prev`, tail recovery and the barrier; `cas` owns content named by its BLAKE3 digest, written through a temporary file; `chain_audit` walks the chain from genesis in waves of at most eight segments and hands back a verdict the writer waits for or halts on; `bundle` exports a city and reads it back, and its manifest is the integrity check.

**Everything else is derived, and deleting it costs a rebuild.** `index` maps `seq` to a segment and a byte offset so one line costs a seek instead of a scan; `hot` is the in-memory view the interface reads without touching disk; `resident` is the byte-budgeted working set read back from disk; `queue` is a projection of enqueued minus consumed; `attribution` splits one billed total so that the cuts reconcile to the microdollar; `verified_prefix` records what a strict walk proved so the next proof hashes instead of re-checking; `snapshot` is what a fold held after one line, named by that line's chain hash; `sessions` writes one slice per room beside the building that ran it.

Three of those touch the disk, and none of them may be believed. A proof record that is missing, damaged, of another rule version or out of step with the chain state entering its segment is not used, and that segment is checked line by line. A snapshot whose line hash does not match the line on disk is folded past, so the fold starts from genesis. Nothing in the product reads a session slice, which a person finds beside the building it belongs to.

**The rest reads or writes git.** `checkpoint` commits a base, a pre-wave and a post-wave checkpoint and refuses a wave whose staged content matches a credential shape; `worktree` gives one node one tree with objects shared and files not; `changes`, `hunks`, `blob` and `status` answer what moved, one file's patch text, one file's bytes at a commit, and what is not committed yet. `reserved` and `alias` are the two policies the write faces share: which bytes belong to a person, and that a junction, symlink or hard link is refused whole.

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:55e5dc2b6535a6661a8196e0390c0111b0f3e04c6ad137cd1da7a02baf75ec2e -->

**One seam.** File operations routed through `vfs` use a `pub(crate)` trait with two adapters: `real_fs` over `std::fs`, holding the handle it is appending through, and `fault_fs`, a deterministic power-loss model. Git operations use `git2` directly, and operations that call `std::fs` directly also bypass this seam; `fault_fs` injects faults only into operations routed through `vfs`, so those exceptions are outside its power-loss tests. The seam stays inner because what a city stores its history on is not a decision this crate delegates outward; [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 4 records it as an inner seam that does not rise to a real one.

<!-- /coderabbit-ai-generated:v1 -->

## When a call fails

`StorageError` is this crate's error type, and `StorageError::into_ax` is its only exit across the crate boundary. Each variant carries the operation, the subject and a recovery, and never the bytes — `SecretEgress` is where that last rule is load-bearing. The codes are the kernel's closed set:

| Code | What reaches it |
|---|---|
| `E_STORAGE_FATAL` | a failed filesystem call, a wave whose barrier failed, a snapshot the disk refused, a repository that would not open |
| `E_LEDGER_HELD` | this city's writer lock is held by another handle |
| `E_LOG_VERSION_UNSUPPORTED` | a Ledger written by a newer version, and non-tail damage, which is never truncated away |
| `E_HISTORY_UNPROVEN` | a line offered before the history this writer opened from is proved |
| `E_CAS_CORRUPT`, `E_PATH_NOT_FOUND` | an object that does not hash to its address, and a locator whose object is gone |
| `E_INVALID_ARGS` | a `seq` the Ledger does not hold, and a range past the end of an object, which is refused rather than clamped |
| `E_OUTSIDE_WRITE_DOMAIN` | a write face handed a name that is an alias |
| `E_SECRET_EGRESS` | a checkpoint of staged content that matches a credential shape; the locations are named and the bytes are not |
| `E_WORKTREE_BUSY`, `E_VERSION_CONFLICT` | a node that already holds a tree or would cross the ceiling, a tree behind the trunk, a merge that would discard uncommitted changes, a create whose name is already taken |
| `E_CONFIG_INVALID` | a bundle that is not the city it claims to be |

Refused rather than attempted: opening a Ledger a newer version wrote, truncating anything that is not the tail, forging a `WriteTarget` from an unchecked path, committing staged content that matches a credential shape, and restoring into an occupied directory.

## Constraints

- Two features, both off by default. `fault` compiles the second `vfs` adapter, so a caller can cut the power under a Ledger: `crates/accounting`'s dev-dependency turns it on for its own tests, and this crate's tests have the adapter without the feature. `conformance` forwards to `kernel/conformance`; this crate's dev-dependency turns that on directly, so its own tests run the port suite either way.
- One writer per city. The Ledger takes an exclusive lock when it opens, so a second process, or a second handle in this process, is refused with `E_LEDGER_HELD` rather than queued behind.
- `Ok` from `append_all` means every line of the wave is written and synced. After a wave fails, the handle refuses every later wave until the Ledger is opened again, and that open is what repairs the tail (`storage D16`).
- Time arrives as a parameter. The crate never reads a wall clock, and the roll threshold and the preallocation arm of a segment are internal: `SEGMENT_ROLL_BYTES` and `SEGMENT_PREALLOCATION` change how the file is cut, never what the history reads as (`storage D31`).
- Reading from a `seq` is the index's answer, not a skip over what came before (`storage D23`).
- `unsafe_code` is forbidden by the workspace lint table. The one platform call that needs more than `std` — a file's link count on Windows — goes through `winapi-util`, whose `unsafe` is inside that crate rather than this one.
- Of this workspace it may name `kernel` and `child`; the code names only `kernel`, the one workspace crate among the seven `cargo tree -p sprawling-storage -e normal --depth 1` prints.
- `digest_cache` is public and has no production reader, so the rule it holds — the same content is digested once for its lifetime — guards no path today (`Spec.lean` §3).

## Who uses it

`cargo tree --workspace -i sprawling-storage -e normal --depth 1` prints five dependents: the product's `sprawling-accounting`, `sprawling-collab`, `sprawling-runtime` and assembly root `sprawling`, and `citysim`, the scenario runner that sits outside the product graph. The fuzz package under `crates/fuzz` names it too, and is a workspace of its own. What each may name is the `depmap` block of [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3.

## Specification

[`Spec.lean`](Spec.lean) is the entry and the parts live under [`spec/`](spec/). [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout.

- §1 the module table with each module's shape and one line, §2 the acceptance criteria, §3 the ten assumptions the design rests on, §7 the module boundary and the negative list.
- §8 the label-to-part table for the interface sections, which keep their `§8-n` labels, and §12 error handling with the table naming the part that holds each `D<n>`.
- §16 the tests that hold what the proofs leave open, §11 the boundaries each open or refuse, and §14 every fixed value with what changing it costs.

What a city keeps on disk, and where, is [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 6. The words are defined in [`docs/glossary.md`](../../docs/glossary.md), among them *Ledger*, *CAS*, *projection*, *checkpoint* and *verified prefix*.

## Checking it

```bash
cargo nextest run -p sprawling-storage --all-features   # the tests, as `just test` runs them
lake build crates.storage.Spec                          # the SPEC's proofs; `just models` builds every crate's
just replay <ledger-dir>                                # the whole-chain audit, run by the binary, read-only
cargo xtask gates spec lexicon specalign                # the SPEC, its vocabulary and its anchors
```

`tests/derived_views.rs` throws away the index and the hot view, rebuilds each from the same Ledger and requires the same answers; `tests/ui/write_target.rs` must fail to compile, because a `WriteTarget` is cleared at one constructor; and the opening of a Ledger is tested against `crates/fixtures/ledger-ahead`, a Ledger written by a newer version, which this crate must refuse rather than read.
