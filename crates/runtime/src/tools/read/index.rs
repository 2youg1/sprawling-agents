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

use std::collections::{BTreeMap, BTreeSet, VecDeque};
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
    map: BTreeMap<PathBuf, Entry>,
    /// The paths a fill is on its way for. Apart from `map`, so a stale
    /// entry can be refilled while it still stands.
    pending: BTreeSet<PathBuf>,
    /// Insertion order of `map`'s keys, for eviction; each key once.
    order: VecDeque<PathBuf>,
    /// The bill of every entry's seek table, in bytes.
    bytes: usize,
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
        let entry = locked.map.get(real)?;
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
        })
    }

    /// A synchronously built entry, from a read that held the whole
    /// file anyway (§8-29-7): no I/O was spent for it.
    pub(super) fn offer(&self, real: PathBuf, entry: Entry) {
        if let Ok(mut locked) = self.locked.lock() {
            insert(&mut locked, real, entry);
        }
    }

    /// Drops the entry for `real`: a read found its seek table no longer
    /// lands on line starts, so the file changed under the same size and
    /// mtime (§8-29-7).
    pub(super) fn forget(&self, real: &Path) {
        if let Ok(mut locked) = self.locked.lock() {
            remove(&mut locked, real);
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
        {
            let Ok(mut locked) = self.locked.lock() else {
                return;
            };
            if locked.pending.len() >= PENDING_CAP || !locked.pending.insert(real.clone()) {
                return;
            }
        }
        let index = Arc::clone(self);
        std::thread::spawn(move || {
            let filled = fill(&real);
            if let Ok(mut locked) = index.locked.lock() {
                locked.pending.remove(&real);
                match filled {
                    Ok(entry) => insert(&mut locked, real, entry),
                    Err(_) => remove(&mut locked, &real),
                }
            }
        });
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
/// first while the bill is over the cap. The newest facts win: an entry
/// that turns out stale misses at `lookup`, whose caller asks for a fill
/// again, so no ordering by mtime is needed — and none could hold, since
/// a file can be replaced by one with an older mtime.
fn insert(locked: &mut Locked, real: PathBuf, entry: Entry) {
    if entry.line_offsets.len() > TABLE_CAP_LINES {
        remove(locked, &real);
        return;
    }
    locked.bytes = locked.bytes.saturating_add(bill(&entry));
    match locked.map.insert(real.clone(), entry) {
        Some(old) => locked.bytes = locked.bytes.saturating_sub(bill(&old)),
        None => locked.order.push_back(real),
    }
    while locked.bytes > TOTAL_CAP_BYTES {
        let Some(oldest) = locked.order.pop_front() else {
            break;
        };
        if let Some(old) = locked.map.remove(&oldest) {
            locked.bytes = locked.bytes.saturating_sub(bill(&old));
        }
    }
}

/// Removes the entry for `real`, its bill, and its place in the order.
fn remove(locked: &mut Locked, real: &Path) {
    if let Some(old) = locked.map.remove(real) {
        locked.bytes = locked.bytes.saturating_sub(bill(&old));
        locked.order.retain(|path| path != real);
    }
}
