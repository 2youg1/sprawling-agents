// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The session slice: a disposable projection of the Ledger, one file per
//! room, laid down beside the building that ran it (memory-SPEC 8-24).
//!
//! **The Ledger is not split.** It stays one chain with one writer, and a
//! slice is derived from it: one file per room address, holding the
//! records the Ledger carries for that address, copied byte for byte
//! from the canonical lines. The product never reads a slice to decide
//! anything; a person standing in a building finds its history beside it,
//! and nothing else consumes it.
//!
//! **One writer, after durability.** [`Sessions::absorb`] is called by
//! [`crate::JsonlLedger`] on the accounting thread once a wave is synced.
//! A refusal is reported and skipped: the history already has the record,
//! and a disposable artifact must never fail history's caller.
//!
//! **The header is the only doubt.** A slice opens with `slices v1 <seq>`,
//! the sequence of its first record. A file whose magic or first sequence
//! disagrees with its header is not the shape this build writes, so it is
//! laid down again from the Ledger rather than migrated. A deleted
//! `sessions/` directory is the same case seen from outside: the next
//! process to touch each room lays its file down again, and the bytes are
//! identical because every line came from the Ledger in the first place.
//!
//! The path comes from `kernel::layout::CityLayout::session_slice`, never
//! from a call site. `xtask slices` holds that sentence: only this module
//! may name the slice path.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, EventRecord, Seq};

use crate::error::{MemoryError, io_err};
use crate::jsonl::{complete_lines, segment_first_seq, segment_names};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

/// The first word of every slice, and the shape this build appends to.
/// Bumped when the shape changes: an older file fails the comparison and
/// is laid down again in silence, which is this module's standing answer
/// to any doubt, so no migration code exists.
pub(crate) const SLICE_MAGIC: &str = "slices v1";

/// The slices over one city's Ledger.
///
/// The filesystem adapter is its own, not the Ledger's: the Ledger's
/// adapter holds the segment handle it appends through, and a projection
/// that took that handle away between every two records would make the
/// city's hot path pay for a side artifact.
pub(crate) struct Sessions {
    layout: CityLayout,
    ledger: PathBuf,
    vfs: RealFs,
    /// One entry per slice this process has written, keyed by its path.
    open: BTreeMap<PathBuf, Open>,
    /// The city's own address, which names the city rather than a room:
    /// a record carrying it is filed nowhere, because no building is
    /// named after the city inside itself.
    city: Option<Address>,
}

/// What one slice file holds, so a later append can continue it.
struct Open {
    addr: Address,
    /// The last ledger sequence the file carries.
    last: Option<Seq>,
}

impl Sessions {
    /// The projection over the city whose Ledger is `dir`, when `dir` is
    /// a city's; `None` for a store somebody opened directly, which has
    /// no buildings to file sessions under.
    pub(crate) fn for_ledger(dir: &Path) -> Option<Sessions> {
        let layout = CityLayout::of_ledger(dir)?;
        let city = layout.city_address();
        Some(Sessions {
            layout,
            ledger: dir.to_path_buf(),
            vfs: RealFs::new(),
            open: BTreeMap::new(),
            city,
        })
    }

    /// Files one durable record into its room's slice.
    ///
    /// A record the city wrote for itself rather than for a room - no
    /// address, the city's own address, or an address inside the
    /// reserved subtree - is filed nowhere, which is not a refusal.
    ///
    /// # Errors
    /// Propagates a slice that cannot be read, laid down or appended. The
    /// caller reports it; the Ledger is already durable either way.
    pub(crate) fn absorb(&mut self, record: &EventRecord) -> Result<(), MemoryError> {
        let Some(addr) = record.addr() else {
            return Ok(());
        };
        if addr.is_reserved() {
            return Ok(());
        }
        if self.city.as_ref().is_some_and(|city| city == addr) {
            return Ok(());
        }
        let path = self.layout.session_slice(addr);
        if !self.open.contains_key(&path) {
            let open = self.attach(addr, &path, record.seq())?;
            self.open.insert(path.clone(), open);
        }
        let up_to_date = self
            .open
            .get(&path)
            .and_then(|open| open.last)
            .is_some_and(|last| last >= record.seq());
        if up_to_date {
            return Ok(());
        }
        let mut line = record
            .canonical_line()
            .map_err(|source| MemoryError::Draft { source })?;
        line.push(b'\n');
        self.vfs
            .append(&path, &line)
            .map_err(io_err("append a session slice", &path))?;
        let open = self.open.get_mut(&path).ok_or_else(|| vanished(&path))?;
        open.last = Some(record.seq());
        Ok(())
    }

    /// Brings the file at `path` up to `tail`, laying it down from the
    /// Ledger when it is missing or not this build's shape.
    fn attach(&mut self, addr: &Address, path: &Path, tail: Seq) -> Result<Open, MemoryError> {
        if !self.vfs.exists(path) {
            return self.lay_down(addr, path, tail);
        }
        let bytes = self
            .vfs
            .read(path)
            .map_err(io_err("read a session slice", path))?;
        let (lines, consumed) = complete_lines(&bytes);
        if consumed < bytes.len() {
            // A tail that stopped mid-line is what a power cut leaves;
            // the bytes after it are not a record. Trim and carry on from
            // the complete prefix, the same reading the Ledger's own
            // recovery takes of a torn segment.
            let len = u64::try_from(consumed).unwrap_or(u64::MAX);
            self.vfs
                .truncate(path, len)
                .map_err(io_err("trim a torn session slice", path))?;
        }
        match resume(addr, &lines) {
            Some(open) => self.catch_up(open, path, tail),
            None => self.lay_down(addr, path, tail),
        }
    }

    /// Appends the records the Ledger holds after what the file already
    /// carries, so a slice left behind by a stopped process catches up
    /// before this one adds to it.
    fn catch_up(&mut self, mut open: Open, path: &Path, tail: Seq) -> Result<Open, MemoryError> {
        let found = self.scan(&open.addr, open.last, tail)?;
        for (seq, line) in &found {
            let mut terminated = Vec::with_capacity(line.len().saturating_add(1));
            terminated.extend_from_slice(line);
            terminated.push(b'\n');
            self.vfs
                .append(path, &terminated)
                .map_err(io_err("catch up a session slice", path))?;
            open.last = Some(*seq);
        }
        if !found.is_empty() {
            self.vfs
                .sync_data(path)
                .map_err(io_err("flush a session slice", path))?;
        }
        Ok(open)
    }

    /// Lays one slice down from the Ledger: the header, then every line
    /// the Ledger holds for this address up to `tail`, in ledger order.
    fn lay_down(&mut self, addr: &Address, path: &Path, tail: Seq) -> Result<Open, MemoryError> {
        let found = self.scan(addr, None, tail)?;
        let Some((first_seq, _)) = found.first() else {
            // Nothing in the Ledger carries this address: no file is a
            // truthful answer, and the next append tries again.
            return Ok(Open {
                addr: addr.clone(),
                last: None,
            });
        };
        let mut body = format!("{SLICE_MAGIC} {}\n", first_seq.value()).into_bytes();
        let mut last = *first_seq;
        for (seq, line) in &found {
            body.extend_from_slice(line);
            body.push(b'\n');
            last = *seq;
        }
        let Some(dir) = path.parent() else {
            return Err(no_parent(path));
        };
        self.vfs
            .create_dir_all(dir)
            .map_err(io_err("create the sessions directory", dir))?;
        if self.vfs.exists(path) {
            self.vfs
                .remove_file(path)
                .map_err(io_err("replace a session slice", path))?;
        }
        self.vfs
            .append(path, &body)
            .map_err(io_err("lay down a session slice", path))?;
        self.vfs
            .sync_data(path)
            .map_err(io_err("flush a session slice", path))?;
        Ok(Open {
            addr: addr.clone(),
            last: Some(last),
        })
    }

    /// The canonical lines of the Ledger that carry `addr`, after `after`
    /// and no later than `through`, in ledger order.
    ///
    /// A segment whose successor begins at or before `after` cannot hold a
    /// newer record and is skipped unread; a line that is not this
    /// build's record grammar cannot carry an address or a sequence, so
    /// it is passed over rather than filed under a guess.
    fn scan(
        &self,
        addr: &Address,
        after: Option<Seq>,
        through: Seq,
    ) -> Result<Vec<(Seq, Vec<u8>)>, MemoryError> {
        let names = segment_names(&self.vfs, &self.ledger)?;
        let mut found = Vec::new();
        for (index, name) in names.iter().enumerate() {
            if !holds_a_later_record(&names, index, after) {
                continue;
            }
            let path = self.ledger.join(name);
            let bytes = self
                .vfs
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

/// What an existing slice claims to hold, or `None` when it is not this
/// build's shape and must be laid down again.
fn resume(addr: &Address, lines: &[&[u8]]) -> Option<Open> {
    let header = lines.first()?;
    let header_seq = parse_header(header)?;
    let mut last: Option<Seq> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let record = EventRecord::parse_line(line).ok()?;
        if record.addr() != Some(addr) {
            return None;
        }
        if index == 1 && record.seq() != header_seq {
            return None;
        }
        if last.is_some_and(|seen| record.seq() <= seen) {
            return None;
        }
        last = Some(record.seq());
    }
    // A header with no record after it is not a slice: there is no
    // sequence to continue from, so the file is laid down again.
    let last = last?;
    Some(Open {
        addr: addr.clone(),
        last: Some(last),
    })
}

/// The sequence in a slice's header, or `None` when the line is not one.
fn parse_header(line: &[u8]) -> Option<Seq> {
    let text = std::str::from_utf8(line).ok()?;
    let digits = text.strip_prefix(SLICE_MAGIC)?.strip_prefix(' ')?;
    if digits.contains(' ') {
        return None;
    }
    digits.parse::<u64>().ok().map(Seq::new)
}

/// The refusal for a slice path that has no directory, which a path from
/// `session_slice` cannot be.
fn no_parent(path: &Path) -> MemoryError {
    MemoryError::Io {
        op: "lay a session slice",
        path: path.to_path_buf(),
        source: io::Error::other("the slice path has no parent directory"),
    }
}

/// The refusal for slice state that vanished from under a reader, which
/// only a defect in this module can produce.
fn vanished(path: &Path) -> MemoryError {
    MemoryError::Io {
        op: "continue a session slice",
        path: path.to_path_buf(),
        source: io::Error::other("the slice state vanished after it was read"),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
