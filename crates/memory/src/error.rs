// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this crate's twelve modules say when persistence refuses, and
//! the one door that turns it into an `AxError`.
//!
//! **Why this is a module of its own.** memory-SPEC 7 recorded
//! the condition when the type was born: it lived beside the ledger
//! while fewer than three modules aggregated here, and moved out at
//! three. Twelve modules import it today, and most of its
//! variants describe failures the ledger cannot produce — a corrupt CAS
//! object, a bundle that is not a city, a worktree that is behind
//! the trunk. Executing a decision whose stated condition has arrived
//! needs no new argument.
//!
//! One rule holds the type together: **a variant carries the operation,
//! the subject and the recovery, and never the bytes**. `SecretEgress`
//! is where that is load-bearing — proving a credential leaked by
//! reprinting it would be the leak.

use std::io;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

/// Crate root error; crosses the crate boundary only via [`MemoryError::into_ax`].
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("{op} failed at {path}: {source}")]
    Io {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("ledger at {path} was written by a newer sprawling (v{v})")]
    VersionAhead { path: PathBuf, v: u64 },
    #[error("ledger segment {path} is damaged at line {line}")]
    Envelope {
        path: PathBuf,
        line: u64,
        #[source]
        source: AxError,
    },
    #[error("could not build a ledger draft")]
    Draft {
        #[source]
        source: AxError,
    },
    #[error("cas object {hash} is not in the store")]
    CasMissing { hash: String },
    #[error("cas object {hash} at {path} does not hash to its address")]
    CasCorrupt { hash: String, path: PathBuf },
    #[error("range does not fit cas object {hash}")]
    RangeOutOfBounds { hash: String },
    #[error("seq {seq} is not in the ledger index")]
    SeqMissing { seq: u64 },
    #[error("checkpoint {op} failed: {detail}")]
    Checkpoint { op: &'static str, detail: String },
    /// Locations only. The matched bytes are never carried — proving a
    /// secret leaked by reprinting it would be the leak.
    #[error("staged content matches secret shapes at {locations}")]
    SecretEgress { locations: String },
    /// An export or restore that is well-formed I/O but not a city:
    /// an occupied destination, a bundle whose manifest disagrees with
    /// what arrived, a chain with a gap.
    #[error("bundle {op} refused: {detail}")]
    Bundle { op: &'static str, detail: String },
    /// A tree that cannot be opened, added or pruned. Environment, not
    /// policy: the repository or the filesystem said no.
    #[error("worktree {op} failed: {detail}")]
    Worktree { op: &'static str, detail: String },
    /// A tree that cannot be granted now: the node already holds one, or
    /// granting it would cross the ceiling. Both are answered by giving
    /// a tree back, which is why they share a code.
    #[error("worktree {name} cannot be opened: {detail}")]
    WorktreeBusy { name: String, detail: String },
    /// A node's work that no longer sits on top of the trunk. Not a
    /// storage fault: the work is intact, and whether it still applies
    /// is the question a machine must not answer by itself.
    #[error("worktree {name} is behind the trunk: {detail}")]
    MergeStale { name: String, detail: String },
    /// A merge that would overwrite or delete a tracked file the person
    /// changed in the city folder and has not committed.
    #[error("merging would discard uncommitted changes to {}", paths.join(", "))]
    MergeWouldDiscard { paths: Vec<String> },
    /// A name that is an alias. The family is refused whole (memory-SPEC
    /// 8-25): a write through one lands where the name does not say.
    #[error("{op} refused at {path}: that name is a {kind}")]
    Alias {
        op: &'static str,
        path: PathBuf,
        kind: crate::alias::AliasKind,
    },
    /// Another handle holds this city's writer lock: a second process,
    /// or a second ledger in this one (memory-SPEC 8-1).
    #[error("the ledger at {dir} is held by another writer")]
    LedgerHeld { dir: PathBuf },
    /// A wave's write or barrier failed, so what the disk holds past
    /// `at` is unknown to this handle; only a reopen can judge it
    /// (memory-SPEC 8-1).
    #[error("the ledger at {} lost its barrier at seq {}", dir.display(), at.value())]
    LedgerBroken { dir: PathBuf, at: kernel::Seq },
    /// A whole-chain audit found a broken line, so this writer stopped
    /// taking new lines; `source` is the audit's own reason (8-27).
    #[error("the ledger stopped taking writes because its chain failed verification")]
    ChainHalted {
        #[source]
        source: AxError,
    },
    /// The disk refused a snapshot read or write. A snapshot is a cache
    /// the Ledger rebuilds, so its advice differs from `Io`'s (8-26).
    #[error("{op} failed at {path}: {source}")]
    Snapshot {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl MemoryError {
    /// The only exit across the crate boundary.
    pub fn into_ax(self) -> AxError {
        match self {
            // Storage write failure is process-fatal; E_STORAGE_FATAL is its loadtime
            // code.
            MemoryError::Io { op, path, source } => {
                AxError::failure(AxCode::StorageFatal, op, path.display().to_string())
                    .with_recovery(format!(
                        "storage failed ({source}); halt cleanly — reopening will tail-truncate"
                    ))
            }
            MemoryError::VersionAhead { path, v } => AxError::failure(
                AxCode::LogVersionUnsupported,
                "open ledger",
                format!("{} (written by a newer sprawling, v{v})", path.display()),
            )
            .with_recovery(format!(
                "open this ledger with the sprawling that wrote it; original path: {}",
                path.display()
            )),
            MemoryError::Envelope { path, line, source } => AxError::failure(
                AxCode::LogVersionUnsupported,
                "open ledger",
                format!("{}:{line}", path.display()),
            )
            .with_recovery(format!(
                "non-tail damage cannot be auto-repaired ({source}); inspect the segment"
            )),
            MemoryError::Draft { source } | MemoryError::ChainHalted { source } => source,
            MemoryError::Worktree { op, detail } => {
                AxError::failure(AxCode::StorageFatal, op, detail).with_recovery(
                    "the repository or the filesystem refused; fix that, then claim again",
                )
            }
            MemoryError::WorktreeBusy { name, detail } => AxError::failure(
                AxCode::WorktreeBusy,
                "open a worktree",
                format!("{name}: {detail}"),
            )
            .with_recovery("release a tree and claim again, or work on a node whose tree is free"),
            MemoryError::MergeStale { name, detail } => AxError::failure(
                AxCode::VersionConflict,
                "merge a node's work",
                format!("{name}: {detail}"),
            )
            .with_recovery(
                "rebuild this node's tree on the trunk as it stands, then have it verified again",
            ),
            MemoryError::MergeWouldDiscard { paths } => AxError::failure(
                AxCode::VersionConflict,
                "merge a node's work",
                format!("uncommitted changes to {}", paths.join(", ")),
            )
            .with_recovery(
                "commit or set aside these changes in the city folder, then merge again",
            ),
            MemoryError::CasMissing { hash } => {
                AxError::failure(AxCode::PathNotFound, "read cas object", hash)
                    .with_recovery("the locator outlived its object; re-materialize or drop it")
            }
            MemoryError::CasCorrupt { hash, path } => AxError::failure(
                AxCode::CasCorrupt,
                "read cas object",
                format!("{hash} at {}", path.display()),
            )
            .with_recovery("bit rot or outside tampering; restore from export or drop the object"),
            MemoryError::RangeOutOfBounds { hash } => {
                AxError::failure(AxCode::InvalidArgs, "read cas range", hash)
                    .with_recovery("range exceeds the object; ask within its size")
            }
            // An absent seq is the caller asking for a line that was
            // never written — not damage, so not a storage fault.
            MemoryError::Snapshot { op, path, source } => {
                AxError::failure(AxCode::StorageFatal, op, path.display().to_string())
                    .with_recovery(format!(
                        "storage failed ({source}); a snapshot is rebuilt from the ledger, so                          remove {} and free the disk; the next start folds from genesis",
                        path.display()
                    ))
            }
            MemoryError::SeqMissing { seq } => {
                AxError::failure(AxCode::InvalidArgs, "read ledger line", seq.to_string())
                    .with_recovery("ask for a seq the ledger actually holds")
            }
            MemoryError::Bundle { op, detail } => {
                AxError::failure(AxCode::ConfigInvalid, op, detail).with_recovery(
                    "restore into an empty directory, and copy the bundle again if it is short",
                )
            }
            MemoryError::Checkpoint { op, detail } => {
                AxError::failure(AxCode::WorktreeBusy, op, detail)
                    .with_recovery("resolve the repository state, then retry the wave")
            }
            MemoryError::SecretEgress { locations } => {
                AxError::failure(AxCode::SecretEgress, "commit checkpoint", locations)
                    .with_recovery("remove the secret from the staged files, then retry")
            }
            MemoryError::LedgerHeld { dir } => AxError::failure(
                AxCode::LedgerHeld,
                "open the ledger for writing",
                dir.display().to_string(),
            )
            .with_recovery(
                "another sprawling process is serving or changing this city; stop it \
                 (Ctrl-C in its terminal, or /quit in its console), then run this again",
            ),
            MemoryError::LedgerBroken { dir, at } => AxError::failure(
                AxCode::StorageFatal,
                "append to the ledger",
                format!("{} at seq {}", dir.display(), at.value()),
            )
            .with_recovery(
                "a write to this ledger failed earlier; free the disk or fix the device, \
                 then restart sprawling so that opening the ledger repairs its tail",
            ),
            MemoryError::Alias { op, path, kind } => {
                AxError::failure(AxCode::OutsideWriteDomain, op, path.display().to_string())
                    .with_recovery(format!(
                        "a {kind} at this name reaches a file other than this name; \
                         replace it with a plain file, then retry"
                    ))
            }
        }
    }
}

/// Wraps a failed filesystem call in the operation and path that were
/// being attempted. Written as a closure so a caller reads
/// `.map_err(io_err("read a bundle file", &path))?` at the call it
/// describes.
pub(crate) fn io_err(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> MemoryError {
    let path = path.to_path_buf();
    move |source| MemoryError::Io { op, path, source }
}
