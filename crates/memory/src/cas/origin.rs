// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whose a block is: the run and the building it was put for, recorded
//! when it is put (memory-SPEC section 8-3).
//!
//! The same bytes may be put for two buildings, so a block keeps every
//! origin it was put for, one line each, in `<dir>/from/<shard>/<hex>`.
//! A line that does not read back - the tail of an append a crash cut -
//! grants nothing, which is the side a read bound fails towards.

use std::path::PathBuf;

use kernel::{Address, B3Hash, RunId};

use super::Cas;
use crate::error::{MemoryError, io_err};

/// The run and the building a block was put for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockOrigin {
    pub run: RunId,
    pub building: Address,
}

impl Cas {
    /// Puts the bytes and records the origin they were put for. `Ok`
    /// means both the object and the record are durable; a second put
    /// with the same origin adds nothing.
    ///
    /// # Errors
    /// `MemoryError::Io` naming the path the store would not take.
    pub fn put_for(&mut self, bytes: &[u8], origin: &BlockOrigin) -> Result<B3Hash, MemoryError> {
        let hash = self.put(bytes)?;
        let line = format!("{} {}", origin.run, origin.building.as_str());
        let (shard_dir, path) = self.origin_path(&hash);
        let mut record = format!("{line}\n");
        if self.vfs.exists(&path) {
            let held = self
                .vfs
                .read(&path)
                .map_err(io_err("read cas origins", &path))?;
            if held
                .split(|byte| *byte == b'\n')
                .any(|recorded| recorded == line.as_bytes())
            {
                return Ok(hash);
            }
            // A crash can cut an append short; the new record starts on
            // a line of its own, or it would read back as part of that tail.
            if held.last().is_some_and(|byte| *byte != b'\n') {
                record.insert(0, '\n');
            }
        }
        self.vfs
            .create_dir_all(&shard_dir)
            .map_err(io_err("create cas origin shard", &shard_dir))?;
        self.vfs
            .append(&path, record.as_bytes())
            .map_err(io_err("record cas origin", &path))?;
        self.vfs
            .sync_data(&path)
            .map_err(io_err("sync cas origin", &path))?;
        self.vfs
            .sync_dir(&shard_dir)
            .map_err(io_err("sync cas origin shard", &shard_dir))?;
        Ok(hash)
    }

    /// Every origin recorded for a block, oldest first; empty for a
    /// block put with no origin and for a hash the store never saw.
    ///
    /// # Errors
    /// `MemoryError::Io` when a record exists and cannot be read.
    pub fn origins(&self, hash: &B3Hash) -> Result<Vec<BlockOrigin>, MemoryError> {
        let (_, path) = self.origin_path(hash);
        if !self.vfs.exists(&path) {
            return Ok(Vec::new());
        }
        let held = self
            .vfs
            .read(&path)
            .map_err(io_err("read cas origins", &path))?;
        Ok(held
            .split(|byte| *byte == b'\n')
            .filter_map(|line| {
                let (run, building) = std::str::from_utf8(line).ok()?.split_once(' ')?;
                Some(BlockOrigin {
                    run: RunId::parse(run).ok()?,
                    building: Address::parse(building).ok()?,
                })
            })
            .collect())
    }

    fn origin_path(&self, hash: &B3Hash) -> (PathBuf, PathBuf) {
        let hex = hash.to_string();
        let shard_dir = self.dir.join("from").join(hex.get(..2).unwrap_or("00"));
        let path = shard_dir.join(&hex);
        (shard_dir, path)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn an_origin_appended_after_a_torn_tail_still_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(dir.path()).unwrap();
        let put_in = |building: &str| BlockOrigin {
            run: RunId::from_bytes([7; 16]),
            building: Address::parse(building).unwrap(),
        };
        let hash = cas.put_for(b"notes\n", &put_in("lab")).unwrap();
        let (_, path) = cas.origin_path(&hash);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"0000 torn")
            .unwrap();

        cas.put_for(b"notes\n", &put_in("vault")).unwrap();

        assert_eq!(
            cas.origins(&hash).unwrap(),
            vec![put_in("lab"), put_in("vault")]
        );
    }
}
