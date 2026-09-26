// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the projection reads from the Ledger's segments: the lines of
//! one address, and the first sequence of every address. Both are the
//! slow path a missing or doubtful slice takes (memory-SPEC 8-24); a
//! room the Ledger never carried before reaches neither.

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{Address, EventRecord, Seq};

use crate::error::{MemoryError, io_err};
use crate::jsonl::{complete_lines, segment_first_seq, segment_names};
use crate::vfs::Vfs;

/// One city's Ledger segments, read through the projection's own adapter.
pub(super) struct Segments<'a> {
    pub(super) vfs: &'a dyn Vfs,
    pub(super) ledger: &'a Path,
}

impl Segments<'_> {
    /// The first sequence of every address in the Ledger's segments.
    pub(super) fn first_seen(&self) -> Result<BTreeMap<Address, Seq>, MemoryError> {
        let (vfs, ledger) = (self.vfs, self.ledger);
        let mut folded = BTreeMap::new();
        for name in segment_names(vfs, ledger)? {
            let path = ledger.join(name);
            let bytes = vfs
                .read(&path)
                .map_err(io_err("read a ledger segment", &path))?;
            let (lines, _) = complete_lines(&bytes);
            for record in lines
                .into_iter()
                .filter_map(|line| EventRecord::parse_line(line).ok())
            {
                if let Some(addr) = record.addr() {
                    folded.entry(addr.clone()).or_insert(record.seq());
                }
            }
        }
        Ok(folded)
    }

    /// The canonical lines of the Ledger that carry `addr`, after `after`
    /// and no later than `through`, in ledger order.
    ///
    /// A segment whose successor begins at or before `after` cannot hold a
    /// newer record and is skipped unread; a line that is not this
    /// build's record grammar cannot carry an address or a sequence, so
    /// it is passed over rather than filed under a guess.
    pub(super) fn lines_of(
        &self,
        addr: &Address,
        after: Option<Seq>,
        through: Seq,
    ) -> Result<Vec<(Seq, Vec<u8>)>, MemoryError> {
        let (vfs, ledger) = (self.vfs, self.ledger);
        let names = segment_names(vfs, ledger)?;
        let mut found = Vec::new();
        for (index, name) in names.iter().enumerate() {
            if !holds_a_later_record(&names, index, after) {
                continue;
            }
            let path = ledger.join(name);
            let bytes = vfs
                .read(&path)
                .map_err(io_err("read a ledger segment", &path))?;
            let (lines, _) = complete_lines(&bytes);
            for line in lines {
                let Ok(record) = EventRecord::parse_line(line) else {
                    continue;
                };
                if record.addr() != Some(addr) {
                    continue;
                }
                if after.is_some_and(|seen| record.seq() <= seen) {
                    continue;
                }
                if record.seq() > through {
                    continue;
                }
                found.push((record.seq(), line.to_vec()));
            }
        }
        Ok(found)
    }
}

/// Whether the segment at `index` can hold a record newer than `after`.
fn holds_a_later_record(names: &[String], index: usize, after: Option<Seq>) -> bool {
    let Some(after) = after else {
        return true;
    };
    match names
        .get(index.saturating_add(1))
        .and_then(|name| segment_first_seq(name))
    {
        Some(next) => next > after,
        // The last segment is always read, and a name this grammar cannot
        // read is read rather than trusted.
        None => true,
    }
}
