// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
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

    #[error("staged content matches known secret shapes at {}", .0.locations())]
    SecretEgress(kernel::event::record::SecretEgressBlocked),

    #[error("bundle {op} refused: {detail}")]
    Bundle { op: &'static str, detail: String },

    #[error("worktree {op} failed: {detail}")]
    Worktree { op: &'static str, detail: String },

    #[error("worktree {name} cannot be opened: {detail}")]
    WorktreeBusy { name: String, detail: String },

    #[error("worktree {name} is behind the trunk: {detail}")]
    MergeStale { name: String, detail: String },

    #[error("merging would discard uncommitted changes to {}", paths.join(", "))]
    MergeWouldDiscard { paths: Vec<String> },

    #[error("{op} refused at {path}: that name is a {kind}")]
    Alias {
        op: &'static str,
        path: PathBuf,
        kind: crate::alias::AliasKind,
    },

    #[error("{} already exists", path.display())]
    NameTaken { path: PathBuf },

    #[error("the ledger at {dir} is held by another writer")]
    LedgerHeld { dir: PathBuf },

    #[error("the ledger at {} lost its barrier at seq {}", dir.display(), at.value())]
    LedgerBroken { dir: PathBuf, at: kernel::Seq },

    #[error("the ledger stopped taking writes because its chain failed verification")]
    ChainHalted {
        #[source]
        source: AxError,
    },

    #[error("the ledger takes no line until the history it opened from is proved")]
    Unproven,

    #[error("{op} failed at {path}: {source}")]
    Snapshot {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl StorageError {

    pub fn into_ax(self) -> AxError {
        match self {

            StorageError::Io { op, path, source } => {
                AxError::failure(AxCode::StorageFatal, op, path.display().to_string())
                    .with_recovery(format!(
                        "storage failed ({source}); halt cleanly — reopening will tail-truncate"
                    ))
            }
            StorageError::VersionAhead { path, v } => AxError::failure(
                AxCode::LogVersionUnsupported,
                "open ledger",
                format!("{} (written by a newer sprawling, v{v})", path.display()),
            )
            .with_recovery(format!(
                "open this ledger with the sprawling that wrote it; original path: {}",
                path.display()
            )),
            StorageError::Envelope { path, line, source } => AxError::failure(
                AxCode::LogVersionUnsupported,
                "open ledger",
                format!("{}:{line}", path.display()),
            )
            .with_recovery(format!(
                "non-tail damage cannot be auto-repaired ({source}); inspect the segment"
            )),
            StorageError::Draft { source } | StorageError::ChainHalted { source } => source,
            StorageError::Unproven => AxError::failure(
                AxCode::HistoryUnproven,
                "append to the ledger",
                "the history this city opened from is still being proved",
            )
            .with_recovery(
                "the city is still proving the history it opened from; send it again once the log says the history is proved",
            ),
            StorageError::Worktree { op, detail } => {
                AxError::failure(AxCode::StorageFatal, op, detail).with_recovery(
                    "the repository or the filesystem refused; fix that, then claim again",
                )
            }
            StorageError::WorktreeBusy { name, detail } => AxError::failure(
                AxCode::WorktreeBusy,
                "open a worktree",
                format!("{name}: {detail}"),
            )
            .with_recovery("release a tree and claim again, or work on a node whose tree is free"),
            StorageError::MergeStale { name, detail } => AxError::failure(
                AxCode::VersionConflict,
                "merge a node's work",
                format!("{name}: {detail}"),
            )
            .with_recovery(
                "rebuild this node's tree on the trunk as it stands, then have it verified again",
            ),
            StorageError::MergeWouldDiscard { paths } => AxError::failure(
                AxCode::VersionConflict,
                "merge a node's work",
                format!("uncommitted changes to {}", paths.join(", ")),
            )
            .with_recovery(
                "commit or set aside these changes in the city folder, then merge again",
            ),
            StorageError::CasMissing { hash } => {
                AxError::failure(AxCode::PathNotFound, "read cas object", hash)
                    .with_recovery("the locator outlived its object; re-materialize or drop it")
            }
            StorageError::CasCorrupt { hash, path } => AxError::failure(
                AxCode::CasCorrupt,
                "read cas object",
                format!("{hash} at {}", path.display()),
            )
            .with_recovery("bit rot or outside tampering; restore from export or drop the object"),
            StorageError::RangeOutOfBounds { hash } => {
                AxError::failure(AxCode::InvalidArgs, "read cas range", hash)
                    .with_recovery("range exceeds the object; ask within its size")
            }
            StorageError::Snapshot { op, path, source } => {
                AxError::failure(AxCode::StorageFatal, op, path.display().to_string())
                    .with_recovery(format!(
                        "storage failed ({source}); a snapshot is rebuilt from the ledger, so \
                         remove {} and free the disk; the next start folds from genesis",
                        path.display()
                    ))
            }

            StorageError::SeqMissing { seq } => {
                AxError::failure(AxCode::InvalidArgs, "read ledger line", seq.to_string())
                    .with_recovery("ask for a seq the ledger actually holds")
            }
            StorageError::Bundle { op, detail } => {
                AxError::failure(AxCode::ConfigInvalid, op, detail).with_recovery(
                    "restore into an empty directory, and copy the bundle again if it is short",
                )
            }
            StorageError::Checkpoint { op, detail } => {
                AxError::failure(AxCode::WorktreeBusy, op, detail)
                    .with_recovery("resolve the repository state, then retry the wave")
            }
            StorageError::SecretEgress(report) => AxError::failure(
                AxCode::SecretEgress,
                "commit checkpoint",
                report.locations(),
            )
            .with_recovery(
                "remove the known key shape from the staged files, then retry; run `sprawling \
                 secrets <city>` to list the recorded positions and reasons without the bytes",
            ),
            StorageError::LedgerHeld { dir } => AxError::failure(
                AxCode::LedgerHeld,
                "open the ledger for writing",
                dir.display().to_string(),
            )
            .with_recovery(
                "another sprawling process is serving or changing this city; close it \
                 (/quit in its terminal, or Ctrl-C where it runs with no console), then run this again",
            ),
            StorageError::LedgerBroken { dir, at } => AxError::failure(
                AxCode::StorageFatal,
                "append to the ledger",
                format!("{} at seq {}", dir.display(), at.value()),
            )
            .with_recovery(
                "a write to this ledger failed earlier; free the disk or fix the device, \
                 then restart sprawling so that opening the ledger repairs its tail",
            ),
            StorageError::Alias { op, path, kind } => {
                AxError::failure(AxCode::OutsideWriteDomain, op, path.display().to_string())
                    .with_recovery(format!(
                        "a {kind} at this name reaches a file other than this name; \
                         replace it with a plain file, then retry"
                    ))
            }
            StorageError::NameTaken { path } => AxError::failure(
                AxCode::VersionConflict,
                "create a file",
                path.display().to_string(),
            )
            .with_recovery(
                "something already stands at this name; read it and change it against its \
                 version, or create the file under another name",
            ),
        }
    }
}

pub(crate) fn io_err(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> StorageError {
    let path = path.to_path_buf();
    move |source| StorageError::Io { op, path, source }
}

#[cfg(test)]
mod tests {
    use super::StorageError;

    #[test]
    fn a_snapshot_recovery_reads_as_one_sentence() {
        let refused = StorageError::Snapshot {
            op: "write a snapshot",
            path: "snap".into(),
            source: std::io::Error::other("disk full"),
        }
        .into_ax();
        assert_eq!(
            refused.recovery(),
            "storage failed (disk full); a snapshot is rebuilt from the ledger, so remove snap \
             and free the disk; the next start folds from genesis"
        );
    }
}
