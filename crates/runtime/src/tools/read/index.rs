// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole-file facts a truncated answer cannot pay for, filled after
//! the answer is already on its way (`crates/runtime/spec/Tools/Read.lean`
//! §8-29-7).
//!
//! A truncated read answers from its window and goes; the total and the
//! version it did not carry are computed here: a whole read of the file
//! once, behind the answer, and the next read of the same file carries
//! them synchronously. The index is a shortcut, never an authority: an
//! entry is used only while the handle's size and mtime say it still
//! describes the file, and a version it wrongly lets through meets
//! `edit`'s own whole-file comparison, which refuses it — the chain is
//! fail-closed.

use std::collections::{BTreeMap, VecDeque};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use kernel::{AxCode, AxError};

/// One file's whole facts. `line_offsets` is the seek table for ranged
/// reads: `line_offsets[i]` is the byte where line `i` starts, counted
/// the way `split_inclusive('\n')` counts lines.
pub(super) struct Entry {
    pub size: u64,
    pub modified: SystemTime,
    pub total_lines: usize,
    pub version: String,
    pub line_offsets: Vec<u64>,
}

/// The most lines one indexed file may carry: eight bytes a line, so
/// one file's seek table never outgrows 2 MiB, and a generated
/// million-line file cannot eat the whole index (§8-29-7).
const TABLE_CAP_LINES: usize = 262_144;

/// The whole index's bill, counted on the seek tables. Past it, the
/// oldest entry leaves (§8-29-7).
const TOTAL_CAP_BYTES: usize = 32 * 1024 * 1024;

/// Files larger than this are not filled: their tables would almost
/// always cross [`TABLE_CAP_LINES`], and their ranged reads stream as
/// before.
const ELIGIBLE_SIZE: u64 = 12 * 1024 * 1024;

/// The most fills one index runs at once. Each holds its whole file
/// while it hashes it, so this and [`ELIGIBLE_SIZE`] bound the memory
/// fills take; a fill asked for past it is not started, and the next
/// truncated read of that file asks again (§8-29-7).
const PENDING_CAP: usize = 2;

#[derive(Default)]
struct Locked {
    map: BTreeMap<PathBuf, Kept>,
    /// The paths a fill is on its way for, each with the [`Seen`] taken
    /// when it was claimed. Apart from `map`, so a stale entry can be
    /// refilled while it still stands.
    pending: BTreeMap<PathBuf, Fill>,
    /// The last [`Seen`] handed out.
    seen: u64,
    /// Insertion order of `map`'s keys, for eviction; each key once.
    order: VecDeque<PathBuf>,
    /// The bill of every entry's seek table, in bytes.
    bytes: usize,
}

/// When a reader looked at a file, in the order the index saw readers
/// start: taken before the file is opened, so a larger `Seen` read a
/// file no older than a smaller one did. Facts replace an entry only
/// when they were seen later, whichever finished first; neither mtime
/// nor size could order them, since a file can be replaced by one with
/// an older mtime and the same size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Seen(u64);

/// One fill on its way: when it was claimed, and whether a `forget`
/// since then voided it. A table a read rejected must not come back
/// from a fill that read the file before the rejection.
struct Fill {
    seen: Seen,
    claim: Claim,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Claim {
    Current,
    Voided,
}

/// One entry and when its facts were seen.
struct Kept {
    seen: Seen,
    entry: Entry,
}

/// The cache itself. One per `ReadTool`, so one per run.
pub(super) struct Index {
    locked: Mutex<Locked>,
}

/// The facts one lookup may use: the two response fields, and the byte
/// the asked line starts at when the caller is about to seek.
pub(super) struct Hit {
    pub total_lines: usize,
    pub version: String,
    pub line_start: Option<u64>,
    /// When the entry's facts were seen, for [`Index::forget`].
    pub seen: Seen,
}

impl Index {
    pub(super) fn new() -> Arc<Index> {
        Arc::new(Index {
            locked: Mutex::new(Locked::default()),
        })
    }

    /// The entry for `real`, when it is ready and still describes the
    /// file the caller just opened: same size, same mtime. `offset` is
    /// the caller's first wanted line; `line_start` carries its byte
    /// position, so a ranged read never pays for the table it does not
    /// read.
    pub(super) fn lookup(
        &self,
        real: &Path,
        size: u64,
        modified: SystemTime,
        offset: u64,
    ) -> Option<Hit> {
        let locked = self.locked.lock().ok()?;
        let kept = locked.map.get(real)?;
        let entry = &kept.entry;
        if entry.size != size || entry.modified != modified {
            return None;
        }
        let line_start = usize::try_from(offset)
            .ok()
            .and_then(|at| entry.line_offsets.get(at).copied());
        Some(Hit {
            total_lines: entry.total_lines,
            version: entry.version.clone(),
            line_start,
            seen: kept.seen,
        })
    }

    /// The [`Seen`] of a reader about to open a file.
    pub(super) fn observe(&self) -> Option<Seen> {
        let mut locked = self.locked.lock().ok()?;
        locked.seen = locked.seen.saturating_add(1);
        Some(Seen(locked.seen))
    }

    /// A synchronously built entry, from a read that held the whole
    /// file anyway (§8-29-7): no I/O was spent for it. Kept unless the
    /// entry standing, or the fill on its way, was seen later.
    pub(super) fn offer(&self, real: PathBuf, entry: Entry, seen: Seen) {
        if let Ok(mut locked) = self.locked.lock() {
            if locked
                .pending
                .get(&real)
                .is_some_and(|fill| fill.seen > seen)
            {
                return;
            }
            insert(&mut locked, real, Kept { seen, entry });
        }
    }

    /// Drops the entry seen at `rejected` for `real`: a read found its
    /// seek table no longer lands on line starts, so the file changed
    /// under the same size and mtime (§8-29-7). A fill claimed before
    /// those facts were seen read no newer a file, so it is voided too;
    /// a later fill, or an entry that has since replaced the rejected
    /// one, may hold the new file and stands.
    pub(super) fn forget(&self, real: &Path, rejected: Seen) {
        if let Ok(mut locked) = self.locked.lock() {
            if let Some(fill) = locked
                .pending
                .get_mut(real)
                .filter(|fill| fill.seen < rejected)
            {
                fill.claim = Claim::Voided;
            }
            if locked
                .map
                .get(real)
                .is_some_and(|kept| kept.seen == rejected)
            {
                remove(&mut locked, real);
            }
        }
    }

    /// Whether a file this size is worth filling at all.
    pub(super) fn eligible(size: u64) -> bool {
        size <= ELIGIBLE_SIZE
    }

    /// Computes the entry behind the answer: one short thread that
    /// reads the file buffered — warming the page cache for the reads
    /// that follow — and writes the entry back. Deduped: one fill per
    /// path at a time, and at most [`PENDING_CAP`] at once. A caller asks
    /// only when its lookup missed, so an entry standing for `real` no
    /// longer describes the file and the fill replaces it. A file that
    /// will not read simply leaves no entry; the reads that asked for it
    /// were already answered.
    pub(super) fn fill_later(self: &Arc<Index>, real: PathBuf) {
        if !self.claim(&real) {
            return;
        }
        let index = Arc::clone(self);
        std::thread::spawn(move || {
            let filled = fill(&real);
            index.settle(real, filled);
        });
    }

    /// Records a fill for `real` as on its way, unless one already is or
    /// [`PENDING_CAP`] are.
    fn claim(&self, real: &Path) -> bool {
        let Ok(mut locked) = self.locked.lock() else {
            return false;
        };
        if locked.pending.len() >= PENDING_CAP || locked.pending.contains_key(real) {
            return false;
        }
        locked.seen = locked.seen.saturating_add(1);
        let seen = Seen(locked.seen);
        locked.pending.insert(
            real.to_path_buf(),
            Fill {
                seen,
                claim: Claim::Current,
            },
        );
        true
    }

    /// Ends the fill for `real`: what it read replaces the entry, or a
    /// file that would not read removes it — unless the entry standing
    /// was seen later than the fill was claimed.
    fn settle(&self, real: PathBuf, filled: Result<Entry, AxError>) {
        let Ok(mut locked) = self.locked.lock() else {
            return;
        };
        let Some(fill) = locked.pending.remove(&real) else {
            return;
        };
        let seen = match fill.claim {
            Claim::Current => fill.seen,
            Claim::Voided => return,
        };
        if locked.map.get(&real).is_some_and(|kept| kept.seen > seen) {
            return;
        }
        match filled {
            Ok(entry) => insert(&mut locked, real, Kept { seen, entry }),
            Err(_) => remove(&mut locked, &real),
        }
    }
}

/// The entry of one file on disk: open it once, take its metadata from
/// the same handle the bytes are read from, hash the bytes, and mark
/// the lines. Buffered, on purpose (D11).
fn fill(real: &Path) -> Result<Entry, AxError> {
    let failed = |err: std::io::Error| {
        AxError::failure(AxCode::StorageFatal, "read", format!("fill of {real:?}: {err}"))
            .with_recovery(
                "the answer this fill was for has already been delivered, and reading the                  file again computes what it wanted synchronously",
            )
    };
    let mut file = std::fs::File::open(real).map_err(failed)?;
    let meta = file.metadata().map_err(failed)?;
    let mut bytes = Vec::new();
    // The file may have grown past what a fill is allowed to hold since
    // the read that asked for it.
    file.by_ref()
        .take(ELIGIBLE_SIZE.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    let modified = meta.modified().map_err(failed)?;
    // The facts are the bytes' only when nothing wrote the file while it
    // was read: the same size and mtime from the same handle afterwards.
    let after = file.metadata().map_err(failed)?;
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read != meta.len() || after.len() != meta.len() || after.modified().ok() != Some(modified) {
        return Err(failed(std::io::ErrorKind::Interrupted.into()));
    }
    let line_offsets = mark(&bytes);
    Ok(Entry {
        size: meta.len(),
        modified,
        total_lines: line_offsets.len(),
        version: crate::tools::version_of(&bytes),
        line_offsets,
    })
}

/// The seek table of one file's bytes: line `i` starts at
/// `offsets[i]`, and the table's length is the line count, counted the
/// way `split_inclusive('\n')` counts — a trailing line without its
/// newline counts, an empty file has none.
fn mark(bytes: &[u8]) -> Vec<u64> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut offsets = vec![0u64];
    for at in memchr::memchr_iter(b'\n', bytes) {
        let next = at.saturating_add(1);
        if next < bytes.len() {
            offsets.push(u64::try_from(next).unwrap_or(u64::MAX));
        }
    }
    offsets
}

/// The bill of one entry's seek table, in bytes.
fn bill(entry: &Entry) -> usize {
    entry.line_offsets.len().saturating_mul(8)
}

/// Inserts or replaces, accounts the table's bytes, and evicts oldest
/// first while the bill is over the cap. Facts seen later win: a stale
/// entry was seen no later than the facts that replace it, unless the
/// caller already declined them ([`Seen`]); an entry that turns out
/// stale misses at `lookup`, whose caller asks for a fill again.
fn insert(locked: &mut Locked, real: PathBuf, kept: Kept) {
    if locked
        .map
        .get(&real)
        .is_some_and(|old| old.seen > kept.seen)
    {
        return;
    }
    if kept.entry.line_offsets.len() > TABLE_CAP_LINES {
        remove(locked, &real);
        return;
    }
    locked.bytes = locked.bytes.saturating_add(bill(&kept.entry));
    match locked.map.insert(real.clone(), kept) {
        Some(old) => locked.bytes = locked.bytes.saturating_sub(bill(&old.entry)),
        None => locked.order.push_back(real),
    }
    while locked.bytes > TOTAL_CAP_BYTES {
        let Some(oldest) = locked.order.pop_front() else {
            break;
        };
        if let Some(old) = locked.map.remove(&oldest) {
            locked.bytes = locked.bytes.saturating_sub(bill(&old.entry));
        }
    }
}

/// Removes the entry for `real`, its bill, and its place in the order.
fn remove(locked: &mut Locked, real: &Path) {
    if let Some(old) = locked.map.remove(real) {
        locked.bytes = locked.bytes.saturating_sub(bill(&old.entry));
        locked.order.retain(|path| path != real);
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
