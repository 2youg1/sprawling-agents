// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

const STAGED: &str = "chain.snap.staged";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSnapshot {
    fold_version: u32,
    seq: Seq,
    line_hash: B3Hash,
    views: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFit {
    Fits,

    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredSnapshot {
    Absent,

    Damaged(String),
    Present(ChainSnapshot),
}

impl ChainSnapshot {
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

    pub fn same_line(&self, other: &ChainSnapshot) -> bool {
        self.seq == other.seq && self.line_hash == other.line_hash
    }

    pub fn fit(&self, line_at_seq: &[u8]) -> SnapshotFit {
        if chain_hash(line_at_seq) == self.line_hash {
            SnapshotFit::Fits
        } else {
            SnapshotFit::Stale
        }
    }

    pub fn resume(&self) -> Result<LineCheck, AxError> {
        Ok(LineCheck::after(self.line_hash, self.seq.next()?))
    }

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

pub fn read_snapshot(dir: &Path) -> Result<StoredSnapshot, StorageError> {
    let file = dir.join(FILE);
    match RealFs::new().read(&file) {
        Ok(bytes) => Ok(ChainSnapshot::decode(&bytes)
            .map_or_else(StoredSnapshot::Damaged, StoredSnapshot::Present)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(StoredSnapshot::Absent),
        Err(e) => Err(refused("read snapshot", &file)(e)),
    }
}

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
