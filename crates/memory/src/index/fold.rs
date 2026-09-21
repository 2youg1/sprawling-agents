// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The folded index: seq to (segment, byte offset), and the scans that
//! build it.
//!
//! Nothing here is persisted. The maps are rebuilt from the segments
//! whenever a reader opens them, and [`crate::index::LedgerIndex`] keeps
//! them for the life of a process so a query costs one directory listing
//! plus the bytes appended since the last look. A side artifact on disk
//! used to carry the same maps; it had no writer left, and a reader of a
//! file nobody writes is a second answer waiting to disagree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kernel::{RunId, Seq};

use crate::error::{MemoryError, io_err};
use crate::jsonl::segment_names;
use crate::vfs::Vfs;

pub(crate) struct Folded {
    pub(crate) entries: BTreeMap<Seq, (String, u64)>,
    pub(crate) runs: BTreeMap<RunId, BTreeSet<Seq>>,
    pub(crate) scanned: BTreeMap<String, u64>,
}

impl Folded {
    pub(crate) fn empty() -> Folded {
        Folded {
            entries: BTreeMap::new(),
            runs: BTreeMap::new(),
            scanned: BTreeMap::new(),
        }
    }

    /// Indexes the complete lines of `tail`, which begins at `from` in
    /// the segment. Returns how many bytes those complete lines took.
    pub(crate) fn fold_segment(&mut self, name: &str, from: u64, tail: &[u8]) -> u64 {
        let mut offset = from;
        let mut complete_bytes = 0u64;
        for line in tail.split_inclusive(|b| *b == b'\n') {
            if line.last().copied() != Some(b'\n') {
                break;
            }
            let body = line.get(..line.len().saturating_sub(1)).unwrap_or(line);
            if !body.is_empty() {
                self.insert_line(name, offset, body);
            }
            let len = u64::try_from(line.len()).unwrap_or(0);
            offset = offset.saturating_add(len);
            complete_bytes = complete_bytes.saturating_add(len);
        }
        complete_bytes
    }

    /// What indexing one line means, in one place: where it sits, and
    /// whose it is. A line that does not parse is skipped rather than
    /// reported — the caller is looking at a ledger that may be damaged,
    /// and an index is how such a ledger gets repaired.
    pub(crate) fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        let Some(located) = locate(body) else {
            return;
        };
        self.entries.insert(located.seq, (name.to_owned(), offset));
        if let Some(run) = located.run {
            self.runs.entry(run).or_default().insert(located.seq);
        }
    }
}

pub(crate) fn rebuild(vfs: &dyn Vfs, dir: &Path) -> Result<Folded, MemoryError> {
    let mut folded = Folded::empty();
    for name in segment_names(vfs, dir)? {
        let path = dir.join(&name);
        let bytes = vfs.read(&path).map_err(io_err("read segment", &path))?;
        let mut offset: u64 = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let complete = line.last().copied() == Some(b'\n');
            let body = if complete {
                line.get(..line.len().saturating_sub(1)).unwrap_or(line)
            } else {
                line
            };
            // A torn tail carries no seq we can trust; it is skipped, and
            // the next append overwrites it (jsonl owns that repair).
            if complete && !body.is_empty() {
                folded.insert_line(&name, offset, body);
            }
            if complete {
                folded.scanned.insert(
                    name.clone(),
                    offset.saturating_add(u64::try_from(line.len()).unwrap_or(0)),
                );
            }
            offset = offset.saturating_add(u64::try_from(line.len()).unwrap_or(0));
        }
        folded.scanned.entry(name).or_insert(0);
    }
    Ok(folded)
}

/// Where a line sits and whose it is.
pub(crate) struct Located {
    pub(crate) seq: Seq,
    /// `None` when the line names no run that reads back as one. The
    /// line is still indexed by seq: an index over a damaged ledger is
    /// exactly what a repair path needs, and a line dropped here would
    /// be invisible to every reader.
    pub(crate) run: Option<RunId>,
}

/// Reads two fields off one parse. Indexing must not depend on the
/// record parsing cleanly as an `EventRecord`, so this asks the raw
/// document rather than the typed one.
pub(crate) fn locate(line: &[u8]) -> Option<Located> {
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    let seq = Seq::new(value.get("seq")?.as_u64()?);
    let run = value
        .get("run")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| RunId::parse(raw).ok());
    Some(Located { seq, run })
}
