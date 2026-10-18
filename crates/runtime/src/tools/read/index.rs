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

/// What the map holds for one path: a fill on its way, or the facts.
enum State {
    Pending,
    Ready(Entry),
}

#[derive(Default)]
struct Locked {
    map: BTreeMap<PathBuf, State>,
    /// Insertion order, for eviction.
    order: VecDeque<PathBuf>,
    /// The bill of every ready entry's seek table, in bytes.
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
        let Some(State::Ready(entry)) = locked.map.get(real) else {
            return None;
        };
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

    /// Whether a file this size is worth filling at all.
    pub(super) fn eligible(size: u64) -> bool {
        size <= ELIGIBLE_SIZE
    }

    /// Computes the entry behind the answer: one short thread that
    /// reads the file buffered — warming the page cache for the reads
    /// that follow — and writes the entry back. Deduped: one fill per
    /// path at a time. A file that will not read simply leaves no
    /// entry; the reads that asked for it were already answered.
    pub(super) fn fill_later(self: &Arc<Index>, real: PathBuf) {
        {
            let Ok(mut locked) = self.locked.lock() else {
                return;
            };
            if locked.map.contains_key(&real) {
                return;
            }
            locked.map.insert(real.clone(), State::Pending);
            locked.order.push_back(real.clone());
        }
        let index = Arc::clone(self);
        std::thread::spawn(move || {
            let filled = fill(&real);
            if let Ok(mut locked) = index.locked.lock() {
                match filled {
                    Ok(entry) => insert(&mut locked, real, entry),
                    Err(_) => {
                        locked.map.remove(&real);
                    }
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
    file.read_to_end(&mut bytes).map_err(failed)?;
    let modified = meta.modified().map_err(failed)?;
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

/// Inserts or replaces, accounts the table's bytes, and evicts oldest
/// first while the bill is over the cap. An entry older than the one
/// standing does not replace it: a fill that read the file before the
/// entry's read does not push the fresher facts out.
fn insert(locked: &mut Locked, real: PathBuf, entry: Entry) {
    if entry.line_offsets.len() > TABLE_CAP_LINES {
        locked.map.remove(&real);
        return;
    }
    if matches!(&locked.map.get(&real), Some(State::Ready(old)) if old.modified > entry.modified) {
        return;
    }
    if let Some(State::Ready(old)) = locked.map.get(&real) {
        locked.bytes = locked
            .bytes
            .saturating_sub(old.line_offsets.len().saturating_mul(8));
    }
    locked.bytes = locked
        .bytes
        .saturating_add(entry.line_offsets.len().saturating_mul(8));
    locked.map.insert(real.clone(), State::Ready(entry));
    if !locked.order.iter().any(|path| path == &real) {
        locked.order.push_back(real);
    }
    while locked.bytes > TOTAL_CAP_BYTES {
        let Some(oldest) = locked.order.pop_front() else {
            break;
        };
        if let Some(State::Ready(old)) = locked.map.remove(&oldest) {
            locked.bytes = locked
                .bytes
                .saturating_sub(old.line_offsets.len().saturating_mul(8));
        }
    }
}
