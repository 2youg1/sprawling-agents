// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One wave of a proof: up to `PROOF_WAVE` segments read whole and their
//! recorded prefixes hashed side by side, before the walk checks the
//! chain through them in order (`crates/storage/spec/ChainAudit.lean` §8-37). What a segment's own
//! bytes decide is all a wave computes; the chain state entering each
//! segment is the walk's.

use std::path::Path;

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, SegmentRecord};
use crate::vfs::Vfs;

/// Segments one wave reads side by side, and so the threads it reads them
/// on, the calling thread being the first (`crates/storage/spec/ChainAudit.lean` §8-37). With
/// `SEGMENT_ROLL_BYTES` it bounds the bytes a proof holds at once.
pub(super) const PROOF_WAVE: usize = 8;

/// Reads the segments of one wave side by side, in their order: the first
/// on this thread, each other one on a scoped thread of its own, every
/// one joined before this returns.
///
/// # Errors
/// `StorageError::Io` when a segment cannot be read, a thread cannot be
/// started, or a thread ends without its segment.
pub(super) fn read_wave(
    dir: &Path,
    wave: &[String],
    records: Option<&ProofRecords>,
    version: u32,
) -> Result<Vec<Read>, StorageError> {
    let Some((first, rest)) = wave.split_first() else {
        return Ok(Vec::new());
    };
    std::thread::scope(|scope| {
        let others = rest
            .iter()
            .map(|name| {
                std::thread::Builder::new()
                    .name("sprawling-proof-read".to_owned())
                    .spawn_scoped(scope, move || Read::ahead(dir, name, records, version))
                    .map(|thread| (name, thread))
                    .map_err(io_err("start a thread to read a segment", &dir.join(name)))
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        std::iter::once(Read::ahead(dir, first, records, version))
            .chain(others.into_iter().map(|(name, thread)| {
                thread.join().unwrap_or_else(|_| {
                    Err(io_err("read a segment on its own thread", &dir.join(name))(
                        std::io::Error::other("the thread ended without the segment"),
                    ))
                })
            }))
            .collect()
    })
}

/// One segment as its wave read it: every byte, and the prefix its record
/// names already hashed when the record may stand in for it.
pub(super) struct Read {
    pub(super) bytes: Vec<u8>,
    pub(super) hashed: Option<Hashed>,
}

/// A segment's record, and a hasher that has hashed exactly the `len`
/// bytes of the prefix it names.
pub(super) struct Hashed {
    pub(super) record: SegmentRecord,
    pub(super) len: usize,
    pub(super) hasher: blake3::Hasher,
}

impl Read {
    /// Reads the segment `name` once and hashes the prefix its record
    /// names (`SegmentRecord::prefix`); the chain state entering the
    /// segment is not known yet and is not needed to hash.
    fn ahead(
        dir: &Path,
        name: &str,
        records: Option<&ProofRecords>,
        version: u32,
    ) -> Result<Read, StorageError> {
        let path = dir.join(name);
        let bytes = RealFs::new()
            .read(&path)
            .map_err(io_err("read a segment to prove", &path))?;
        let hashed = records
            .and_then(|records| records.read(name))
            .and_then(|record| {
                let prefix = record.prefix(&bytes, version)?;
                let mut hasher = blake3::Hasher::new();
                hasher.update(prefix);
                Some(Hashed {
                    len: prefix.len(),
                    record,
                    hasher,
                })
            });
        Ok(Read { bytes, hashed })
    }
}
