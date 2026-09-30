// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The session slice: a disposable projection of the Ledger, one file per
//! room, laid down beside the building that ran it (storage-SPEC 8-24).
//!
//! **The Ledger is not split.** It stays one chain with one writer, and a
//! slice is derived from it: one file per room address, holding the
//! records the Ledger carries for that address, copied byte for byte
//! from the canonical lines. The product never reads a slice to decide
//! anything; a person standing in a building finds its history beside it,
//! and nothing else consumes it.
//!
//! **One writer, after durability.** [`Sessions::absorb`] is called by
//! [`crate::JsonlLedger`] on the accounting thread once a wave is synced,
//! or, once the ledger has handed the slices off, by the thread that
//! took them - a served city's view thread (storage-SPEC 8-24).
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
//! The path is derived here and nowhere else: [`session_slice`] is private
//! to this module, so no other module can name where a slice lies, and a
//! reader of a projection that is deleted and rebuilt cannot be written.
//! The one fact another module needs - whether a path is a slice, so the
//! checkpoint never stages one - is [`is_session_projection`].

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, EventRecord, RESERVED_PREFIX, Seq};

use crate::error::{StorageError, io_err};
use crate::jsonl::complete_lines;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

mod from_ledger;

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
pub struct Sessions {
    layout: CityLayout,
    ledger: PathBuf,
    vfs: RealFs,
    /// One entry per slice this process has written, keyed by its path.
    open: BTreeMap<PathBuf, Open>,
    /// The city's own address, which names the city rather than a room:
    /// a record carrying it is filed nowhere, because no building is
    /// named after the city inside itself.
    city: Option<Address>,
    /// The first ledger sequence of every address the Ledger carries,
    /// folded from the segments the first time a slice is missing and
    /// kept current by every absorb, so a room the Ledger never carried
    /// before is filed from its one record without reading a segment.
    first_seen: Option<BTreeMap<Address, Seq>>,
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
            first_seen: None,
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
    pub fn absorb(&mut self, record: &EventRecord) -> Result<(), StorageError> {
        let Some(addr) = record.addr() else {
            return Ok(());
        };
        if addr.is_reserved() {
            return Ok(());
        }
        if self.city.as_ref().is_some_and(|city| city == addr) {
            return Ok(());
        }
        if let Some(first_seen) = self.first_seen.as_mut() {
            first_seen.entry(addr.clone()).or_insert(record.seq());
        }
        let path = session_slice(&self.layout, addr);
        if !self.open.contains_key(&path) {
            let open = self.attach(addr, &path, record)?;
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
            .map_err(|source| StorageError::Draft { source })?;
        line.push(b'\n');
        self.vfs
            .append(&path, &line)
            .map_err(io_err("append a session slice", &path))?;
        let open = self.open.get_mut(&path).ok_or_else(|| vanished(&path))?;
        open.last = Some(record.seq());
        Ok(())
    }

    /// Brings the file at `path` up to `record`, laying it down from the
    /// Ledger when it is missing or not this build's shape; a missing
    /// file for an address `record` is the first of is laid down from
    /// `record` alone.
    fn attach(
        &mut self,
        addr: &Address,
        path: &Path,
        record: &EventRecord,
    ) -> Result<Open, StorageError> {
        let tail = record.seq();
        if !self.vfs.exists(path) {
            return match self.first_seq_of(addr, tail)? {
                Some(first) if first < tail => self.lay_down(addr, path, tail),
                Some(_) | None => {
                    let line = record
                        .canonical_line()
                        .map_err(|source| StorageError::Draft { source })?;
                    self.write_slice(addr, path, &[(tail, line)])
                }
            };
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
    fn catch_up(&mut self, mut open: Open, path: &Path, tail: Seq) -> Result<Open, StorageError> {
        let found = self.segments().lines_of(&open.addr, open.last, tail)?;
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

    /// The first sequence the Ledger carries for `addr`, folding every
    /// address once when this process has not yet read the segments.
    ///
    /// `tail` is the record being absorbed, which is already durable and
    /// so already in the fold; an answer equal to it means the Ledger
    /// holds nothing earlier for this address.
    fn first_seq_of(&mut self, addr: &Address, tail: Seq) -> Result<Option<Seq>, StorageError> {
        if self.first_seen.is_none() {
            let mut folded = self.segments().first_seen()?;
            folded.entry(addr.clone()).or_insert(tail);
            self.first_seen = Some(folded);
        }
        Ok(self
            .first_seen
            .as_ref()
            .and_then(|folded| folded.get(addr))
            .copied())
    }

    fn segments(&self) -> from_ledger::Segments<'_> {
        from_ledger::Segments {
            vfs: &self.vfs,
            ledger: &self.ledger,
        }
    }

    /// Lays one slice down from the Ledger: the header, then every line
    /// the Ledger holds for this address up to `tail`, in ledger order.
    fn lay_down(&mut self, addr: &Address, path: &Path, tail: Seq) -> Result<Open, StorageError> {
        let found = self.segments().lines_of(addr, None, tail)?;
        self.write_slice(addr, path, &found)
    }

    /// Writes one slice from `found`, the lines the Ledger holds for
    /// `addr` in ledger order, replacing whatever the file held.
    fn write_slice(
        &mut self,
        addr: &Address,
        path: &Path,
        found: &[(Seq, Vec<u8>)],
    ) -> Result<Open, StorageError> {
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
        for (seq, line) in found {
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

/// One building's projection of the sessions run in it, under that
/// building's reserved subtree.
const SESSIONS_DIR: &str = "sessions";

/// The projection of one session's records: the room address is the
/// session's identity, and a building is its first segment.
///
/// An address below a building becomes one file under that building's
/// sessions, its name the rooms below the building, so the nesting of
/// rooms is the nesting of the files and no reader has to know which
/// segment was a building. A run that named no session works at the
/// building's own address, and its file then carries the building's name.
fn session_slice(layout: &CityLayout, room: &Address) -> PathBuf {
    let raw = room.as_str();
    let (building, under) = raw.split_once('/').unwrap_or((raw, ""));
    let name = if under.is_empty() { building } else { under };
    let mut path = layout.root().join(building);
    path.push(RESERVED_PREFIX);
    path.push(SESSIONS_DIR);
    path.push(format!("{name}.jsonl"));
    path
}

/// Whether `relative` names a session slice: a file under some scope's
/// reserved `sessions` directory.
///
/// The city appends to a slice while a wave runs, so the checkpoint never
/// stages one: git reads a workdir file it believes it knows, and a file
/// the city itself keeps writing makes that read refuse the whole wave.
/// The reserved prefix is required immediately before `sessions`, so a
/// person's own directory that happens to carry that name is not mistaken
/// for the projection.
pub(crate) fn is_session_projection(relative: &Path) -> bool {
    let mut held = false;
    for component in relative.components() {
        let Some(name) = component.as_os_str().to_str() else {
            held = false;
            continue;
        };
        if name.eq_ignore_ascii_case(SESSIONS_DIR) {
            return held;
        }
        held = name.eq_ignore_ascii_case(RESERVED_PREFIX);
    }
    false
}

/// The refusal for a slice path that has no directory, which a path from
/// [`session_slice`] cannot be.
fn no_parent(path: &Path) -> StorageError {
    StorageError::Io {
        op: "lay a session slice",
        path: path.to_path_buf(),
        source: io::Error::other("the slice path has no parent directory"),
    }
}

/// The refusal for slice state that vanished from under a reader, which
/// only a defect in this module can produce.
fn vanished(path: &Path) -> StorageError {
    StorageError::Io {
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
