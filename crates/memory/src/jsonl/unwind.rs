// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A wave that failed to reach the disk leaves nothing the next wave
//! builds on (memory-SPEC 8-1).

use std::path::PathBuf;

use crate::error::{MemoryError, io_err};

use super::ledger::JsonlLedger;

impl JsonlLedger {
    /// Appends one wave's bytes and makes them durable. On failure the
    /// segments go back to their length before the wave; a restore that
    /// fails too stays pending for [`Self::finish_unwind`].
    pub(crate) fn write_wave(
        &mut self,
        writes: &[(PathBuf, Vec<u8>)],
        created: Vec<PathBuf>,
    ) -> Result<(), MemoryError> {
        let written = self.land(writes, &created);
        if written.is_err() {
            self.pending_unwind = Some(created);
            if let Err(unwind) = self.finish_unwind() {
                eprintln!("the ledger tail is restored before the next wave instead: {unwind}");
            }
        }
        written
    }

    /// Completes a restore an earlier failed wave left pending. Called
    /// before a wave decides which segments it creates, because a pending
    /// restore may remove the segment the wave would append to.
    pub(crate) fn finish_unwind(&mut self) -> Result<(), MemoryError> {
        let Some(created) = self.pending_unwind.take() else {
            return Ok(());
        };
        let restored = self.restore(&created);
        if restored.is_err() {
            self.pending_unwind = Some(created);
        }
        restored
    }

    fn land(
        &mut self,
        writes: &[(PathBuf, Vec<u8>)],
        created: &[PathBuf],
    ) -> Result<(), MemoryError> {
        for (path, bytes) in writes {
            self.vfs
                .append(path, bytes)
                .map_err(io_err("append event line", path))?;
        }
        for (path, _) in writes {
            self.vfs
                .sync_data(path)
                .map_err(io_err("sync segment", path))?;
        }
        if !created.is_empty() {
            self.vfs
                .sync_dir(&self.dir)
                .map_err(io_err("sync ledger dir", &self.dir))?;
        }
        Ok(())
    }

    fn restore(&mut self, created: &[PathBuf]) -> Result<(), MemoryError> {
        for path in created {
            if self.vfs.exists(path) {
                self.vfs
                    .remove_file(path)
                    .map_err(io_err("remove a segment the failed wave created", path))?;
            }
        }
        if !created.contains(&self.seg_path) {
            self.vfs
                .truncate(&self.seg_path, self.seg_len)
                .map_err(io_err(
                    "truncate a segment back before the failed wave",
                    &self.seg_path,
                ))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "unwind/tests.rs"]
mod tests;
