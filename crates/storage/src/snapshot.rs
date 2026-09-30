// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The chain-hash snapshot: what a fold held after one line of the
//! Ledger, with that line's seq and chain hash (storage-SPEC 8-26).
//!
//! A projection, never history: a start that finds no snapshot, a
//! damaged one, or one whose line hash does not match the line on disk
//! folds the whole ledger, as it did before snapshots existed. That
//! resuming is sound at all is `crates/storage/spec/Snapshot.lean`'s
//! `resumeIsWhole`.

use std::path::Path;

use kernel::ledger::chain_hash;
use kernel::{AxError, B3Hash, Seq};

use crate::error::StorageError;
use crate::jsonl::LineCheck;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

mod start;

pub use start::{SnapshotStart, WholeFold, start_from_snapshot, tail_after};

const MAGIC: &[u8; 8] = b"SPRSNAP1";
const FILE: &str = "chain.snap";
/// Written whole and synced here first, then renamed over [`FILE`], so a
/// write torn by a crash never leaves half a snapshot under that name.
const STAGED: &str = "chain.snap.staged";

/// What a fold held after the line at `seq`, and the chain hash that
/// names that line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSnapshot {
    fold_version: u32,
    seq: Seq,
    line_hash: B3Hash,
    views: Vec<u8>,
}

/// Whether the line a positioned read found at the snapshot's seq is
/// the line the snapshot was cut at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFit {
    Fits,
    /// Another line sits at that seq: the ledger is not the one the
    /// snapshot was cut from, so its state must not be resumed.
    Stale,
}

/// What the snapshot directory holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredSnapshot {
    Absent,
    /// Bytes that are not a whole snapshot, with the reason; a caller
    /// discards them and folds the whole ledger.
    Damaged(String),
    Present(ChainSnapshot),
}

impl ChainSnapshot {
    /// Cut a snapshot after `line`, the raw line (without its `\n`) at `seq`.
    pub fn cut(fold_version: u32, seq: Seq, line: &[u8], views: Vec<u8>) -> ChainSnapshot {
        ChainSnapshot {
            fold_version,
            seq,
            line_hash: chain_hash(line),
            views,
        }
    }

    pub fn fold_version(&self) -> u32 {
        self.fold_version
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn views(&self) -> &[u8] {
        &self.views
    }

    /// Whether `other` was cut at the same line of the same ledger: the
    /// same seq and the same chain hash, whatever each fold holds.
    pub fn same_line(&self, other: &ChainSnapshot) -> bool {
        self.seq == other.seq && self.line_hash == other.line_hash
    }

    /// Judge the raw line a positioned read found at [`Self::seq`].
    pub fn fit(&self, line_at_seq: &[u8]) -> SnapshotFit {
        if chain_hash(line_at_seq) == self.line_hash {
            SnapshotFit::Fits
        } else {
            SnapshotFit::Stale
        }
    }

    /// The chain state after the snapshot's line: the next line must
    /// carry `prev = line_hash` and the seq after `seq`.
    ///
    /// # Errors
    /// What `Seq::next` says when `seq` is the last seq there is.
    pub fn resume(&self) -> Result<LineCheck, AxError> {
        Ok(LineCheck::after(self.line_hash, self.seq.next()?))
    }

    /// `MAGIC | fold_version u32 LE | seq u64 LE | line_hash | blake3(views) | views`.
    fn encode(&self) -> Vec<u8> {
        let digest = B3Hash::digest(&self.views);
        [
            MAGIC.as_slice(),
            &self.fold_version.to_le_bytes(),
            &self.seq.value().to_le_bytes(),
            self.line_hash.as_bytes(),
            digest.as_bytes(),
            &self.views,
        ]
        .concat()
    }

    fn decode(bytes: &[u8]) -> Result<ChainSnapshot, String> {
        let short = || "the file ends inside its header".to_owned();
        let (magic, rest) = bytes.split_first_chunk::<8>().ok_or_else(short)?;
        if magic != MAGIC {
            return Err("the file does not start with the snapshot magic".to_owned());
        }
        let (fold_version, rest) = rest.split_first_chunk::<4>().ok_or_else(short)?;
        let (seq, rest) = rest.split_first_chunk::<8>().ok_or_else(short)?;
        let (line_hash, rest) = rest.split_first_chunk::<32>().ok_or_else(short)?;
        let (digest, views) = rest.split_first_chunk::<32>().ok_or_else(short)?;
        if B3Hash::digest(views).as_bytes() != digest {
            return Err("the views do not hash to the digest written beside them".to_owned());
        }
        Ok(ChainSnapshot {
            fold_version: u32::from_le_bytes(*fold_version),
            seq: Seq::new(u64::from_le_bytes(*seq)),
            line_hash: B3Hash::from_bytes(*line_hash),
            views: views.to_vec(),
        })
    }
}

/// Write `snapshot` into `dir` (created when absent), replacing the one
/// there.
///
/// # Errors
/// `StorageError::Snapshot` naming the step that failed.
pub fn write_snapshot(dir: &Path, snapshot: &ChainSnapshot) -> Result<(), StorageError> {
    let mut vfs = RealFs::new();
    let (file, staged) = (dir.join(FILE), dir.join(STAGED));
    vfs.create_dir_all(dir)
        .map_err(refused("create snapshot dir", dir))?;
    if vfs.exists(&staged) {
        vfs.remove_file(&staged)
            .map_err(refused("remove staged snapshot", &staged))?;
    }
    vfs.append(&staged, &snapshot.encode())
        .map_err(refused("write staged snapshot", &staged))?;
    vfs.sync_data(&staged)
        .map_err(refused("sync staged snapshot", &staged))?;
    vfs.rename(&staged, &file)
        .map_err(refused("rename staged snapshot", &file))?;
    vfs.sync_dir(dir).map_err(refused("sync snapshot dir", dir))
}

/// Read the snapshot `dir` holds.
///
/// # Errors
/// `StorageError::Snapshot` when the file exists and cannot be read; bytes that
/// are not a snapshot are `StoredSnapshot::Damaged`, not an error.
pub fn read_snapshot(dir: &Path) -> Result<StoredSnapshot, StorageError> {
    let file = dir.join(FILE);
    match RealFs::new().read(&file) {
        Ok(bytes) => Ok(ChainSnapshot::decode(&bytes)
            .map_or_else(StoredSnapshot::Damaged, StoredSnapshot::Present)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(StoredSnapshot::Absent),
        Err(e) => Err(refused("read snapshot", &file)(e)),
    }
}

/// The constructor of a snapshot's I/O failure, as `io_err` is the
/// ledger's.
fn refused(op: &'static str, path: &Path) -> impl FnOnce(std::io::Error) -> StorageError {
    let path = path.to_path_buf();
    move |source| StorageError::Snapshot { op, path, source }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
