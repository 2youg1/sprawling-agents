// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, SegmentRecord};
use crate::vfs::Vfs;

pub(super) const PROOF_WAVE: usize = 8;

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

pub(super) struct Read {
    pub(super) bytes: Vec<u8>,
    pub(super) hashed: Option<Hashed>,
}

pub(super) struct Hashed {
    pub(super) record: SegmentRecord,
    pub(super) len: usize,
    pub(super) hasher: blake3::Hasher,
}

impl Read {
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
