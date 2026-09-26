// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger types: the struct, its reports, and segment grammar.

use std::path::{Path, PathBuf};

use kernel::{B3Hash, EventRecord, Seq};

use crate::error::{MemoryError, io_err};
use crate::vfs::Vfs;

/// Segment rolling threshold. Internal affair: changing it changes how
/// files are cut, never any observable semantics (memory-SPEC 14).
pub(crate) const SEGMENT_ROLL_BYTES: u64 = 64 * 1024 * 1024;

/// What open found and repaired.
pub struct OpenReport {
    pub recovered: Option<TailTruncation>,
}

pub struct TailTruncation {
    pub dropped_bytes: u64,
}

/// The durable Ledger. See the module doc for the contract.
pub struct JsonlLedger {
    pub(crate) vfs: Box<dyn Vfs>,
    pub(crate) dir: PathBuf,
    pub(crate) seg_path: PathBuf,
    pub(crate) seg_len: u64,
    pub(crate) next_seq: Seq,
    pub(crate) prev: B3Hash,
    pub(crate) roll_bytes: u64,
    /// Whether `seg_len`, `next_seq` and `prev` still name the end of
    /// the segments; a failed wave leaves it broken until a reopen.
    pub(crate) barrier: super::barrier::Barrier,
    /// The per-room projection of what this ledger appends, laid down
    /// after a wave is durable. `None` for a ledger opened directly in a
    /// directory that is not a city's, which has no buildings to file
    /// sessions under.
    pub(crate) sessions: Option<crate::sessions::Sessions>,
    /// The write-path observer, called only after the wave is durable.
    pub(crate) observer: Option<WriteObserver>,
    /// This process's hold on the city's writer lock. `None` for a
    /// directory that is not a city's ledger, and on the fault model,
    /// whose disk no other process can reach.
    pub(crate) lock: Option<WriterLock>,
    /// The stop a failed whole-chain audit trips (memory-SPEC 8-27).
    pub(crate) halt: crate::chain_audit::ChainHalt,
    /// The segments a failed wave created, while restoring the disk to
    /// its length before that wave is still owed (`jsonl::unwind`).
    pub(crate) pending_unwind: Option<Vec<PathBuf>>,
}

/// This process's exclusive hold on a city's Ledger, released when the
/// ledger that took it is dropped (memory-SPEC 8-1).
///
/// The operating system keeps the lock on an open handle and refuses it
/// to every other handle on the same file, in this process or in
/// another, which is what makes "a city has one writer" hold across
/// processes rather than only inside the one that owns the type. The
/// file is never removed: a lock file deleted on release would let a
/// latecomer lock a new file of the same name while the earlier holder
/// still held the old one.
pub(crate) struct WriterLock {
    _held: std::fs::File,
}

impl WriterLock {
    /// Takes the writer lock of the ledger directory `dir`: the file
    /// `<name>.lock` beside it, named from `dir` alone so that this
    /// module, not the city layout, owns where the lock lies
    /// (memory-SPEC 12).
    ///
    /// # Errors
    /// `LedgerHeld` when another handle holds the lock; `Io` when `dir`
    /// has no name to put a sibling beside (a root, `..`), or the lock
    /// file cannot be made, or the lock cannot be asked for.
    pub(crate) fn take(dir: &Path) -> Result<WriterLock, MemoryError> {
        let name = dir.file_name().ok_or_else(|| {
            io_err("name the ledger lock", dir)(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "a ledger directory needs a name for its lock to sit beside it",
            ))
        })?;
        let mut lock_name = name.to_os_string();
        lock_name.push(".lock");
        let path = dir.with_file_name(lock_name);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(io_err("create the ledger's parent directory", parent))?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(io_err("open the ledger lock", &path))?;
        match file.try_lock() {
            Ok(()) => Ok(WriterLock { _held: file }),
            Err(std::fs::TryLockError::WouldBlock) => Err(MemoryError::LedgerHeld {
                dir: dir.to_path_buf(),
            }),
            Err(std::fs::TryLockError::Error(source)) => {
                Err(io_err("lock the ledger", &path)(source))
            }
        }
    }
}

/// The write-path observer: what a live reader (the control surface's
/// event stream) installs to hear each line once it is durable.
pub type WriteObserver = Box<dyn FnMut(&EventRecord) + Send>;

/// Chain state at the entrance of the last segment.
pub(crate) struct TailBoundary {
    pub(crate) prev: B3Hash,
    pub(crate) next_seq: Seq,
    pub(crate) prior: Option<PriorSegment>,
}

pub(crate) struct PriorSegment {
    pub(crate) path: PathBuf,
    pub(crate) len: u64,
}

/// The two names a segment is made of. One home for the grammar: the
/// writer that names a segment and the readers that recognise one, or
/// read a sequence back off one, agree because they read these.
const SEGMENT_PREFIX: &str = "ledger-";
const SEGMENT_SUFFIX: &str = ".jsonl";

pub(crate) fn segment_file_name(first_seq: Seq) -> String {
    format!("{SEGMENT_PREFIX}{:020}{SEGMENT_SUFFIX}", first_seq.value())
}

pub(crate) fn is_segment(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with(SEGMENT_PREFIX) && name.ends_with(SEGMENT_SUFFIX),
        None => false,
    }
}

/// The first sequence a segment's name claims, for a reader that wants
/// only the records after a sequence it already holds: it skips the
/// segments that cannot contain them by reading the name, rather than by
/// opening the file. `None` for a name that is not this grammar.
///
/// The inverse of [`segment_file_name`], and it lives beside it for the
/// reason the prefix and suffix are constants here: one reader of a
/// grammar, not two spellings of it.
pub(crate) fn segment_first_seq(name: &str) -> Option<Seq> {
    let digits = name
        .strip_prefix(SEGMENT_PREFIX)?
        .strip_suffix(SEGMENT_SUFFIX)?;
    digits.parse::<u64>().ok().map(Seq::new)
}

/// The segment files of `dir`, in the ledger's own order.
///
/// `Vfs::list` answers files only, already sorted: zero-padded names
/// sort lexically the way they sort numerically, so the order is the
/// ledger's own rather than the filesystem's.
pub(crate) fn segment_names(vfs: &dyn Vfs, dir: &Path) -> Result<Vec<String>, MemoryError> {
    let mut names = Vec::new();
    for path in vfs.list(dir).map_err(io_err("list ledger dir", dir))? {
        if !is_segment(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

/// Complete (`\n`-terminated) lines and the leftover tail bytes.
pub(crate) fn complete_lines(bytes: &[u8]) -> (Vec<&[u8]>, usize) {
    let mut lines = Vec::new();
    let mut consumed = 0usize;
    let mut start = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            if let Some(line) = bytes.get(start..index) {
                lines.push(line);
            }
            start = index.saturating_add(1);
            consumed = start;
        }
    }
    (lines, consumed)
}
