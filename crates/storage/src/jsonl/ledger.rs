// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger types: the struct, its reports, and segment grammar.

use std::io;
use std::path::{Path, PathBuf};

use kernel::{B3Hash, EventRecord, Seq};

use crate::error::{StorageError, io_err};
use crate::vfs::Vfs;

/// Segment rolling threshold. Internal affair: changing it changes how
/// files are cut, never any observable semantics (`crates/storage/Spec.lean` §14).
pub(crate) const SEGMENT_ROLL_BYTES: u64 = 64 * 1024 * 1024;

/// How a segment's file is sized while it is written (storage D31).
///
/// Both arms keep the same durability and the same history on disk; an
/// arm changes only what the barrier has to flush. The preallocated
/// segment ends in zero bytes past its last record, which the reader
/// strips (`crates/storage/spec/Jsonl/Preallocate.lean`), so the writer's
/// position is the end of the records, never the end of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentPreallocation {
    /// The file grows by one append at its end per wave.
    Grow,
    /// A new segment is set to the roll size (`File::set_len`) when it is
    /// created, and each wave writes at the end of its records, inside
    /// that length: `SetFileInformationByHandle(FileEndOfFileInfo)` on
    /// Windows, `ftruncate` on Linux and macOS.
    ToRollSize,
}

impl SegmentPreallocation {
    /// The length `open` leaves the last segment at, given the end of its
    /// records (`keep`), the bytes of tear it drops after them
    /// (`dropped`, zeros not counted) and its length on disk (`total`).
    /// A grown segment ends at its records, because the next append writes
    /// at the end of the file. A preallocated one keeps its zero tail when
    /// nothing was torn, because the next wave writes at `keep` anyway.
    pub(crate) fn length_after_open(self, keep: u64, dropped: u64, total: u64) -> u64 {
        match (self, dropped) {
            (SegmentPreallocation::ToRollSize, 0) => total,
            (SegmentPreallocation::ToRollSize | SegmentPreallocation::Grow, _) => keep,
        }
    }
}

/// The arm every ledger this build opens writes with. It stays `Grow`
/// until the release-build reading of the barrier with each arm (storage
/// D31) chooses; choosing is this one value, on Windows, macOS and Linux
/// alike.
pub(crate) const SEGMENT_PREALLOCATION: SegmentPreallocation = SegmentPreallocation::Grow;

/// Where the records in `segment` end: its length without the run of
/// zero bytes a preallocated segment holds past its last record. A zero
/// byte is space no write has reached yet, never a byte of a record, so
/// `open` scans the last segment up to here and the side index stops
/// reading at the first window whose records end before the window does
/// (`crates/storage/spec/Jsonl/Preallocate.lean`, storage D31 and D32).
pub(crate) fn records_end(segment: &[u8]) -> usize {
    segment
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |at| at.saturating_add(1))
}

/// What open found and repaired, and what its tail recovery read and
/// checked (`crates/storage/spec/Jsonl.lean` §8-34).
pub struct OpenReport {
    pub recovered: Option<TailTruncation>,
    pub counted: crate::ProofCount,
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
    /// How this ledger sizes its segments; [`SEGMENT_PREALLOCATION`]
    /// outside tests.
    pub(crate) preallocation: SegmentPreallocation,
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
    /// The stop a failed whole-chain audit trips (`crates/storage/spec/Worktree/Back.lean` §8-27).
    pub(crate) halt: crate::chain_audit::ChainHalt,
    /// The segments a failed wave created, while restoring the disk to
    /// its length before that wave is still owed (`jsonl::unwind`).
    pub(crate) pending_unwind: Option<Vec<PathBuf>>,
}

/// This process's exclusive hold on a city's Ledger, released when the
/// ledger that took it is dropped (`crates/storage/spec/Jsonl.lean` §8-1).
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
    /// (storage D18).
    ///
    /// # Errors
    /// `LedgerHeld` when another handle holds the lock; `Io` when `dir`
    /// has no name to put a sibling beside (a root, `..`), or the lock
    /// file cannot be made, or the lock cannot be asked for.
    pub(crate) fn take(dir: &Path) -> Result<WriterLock, StorageError> {
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
            Err(std::fs::TryLockError::WouldBlock) => Err(StorageError::LedgerHeld {
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
pub(crate) fn segment_names(vfs: &dyn Vfs, dir: &Path) -> Result<Vec<String>, StorageError> {
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

/// A count of segment bytes or lines as the `u64` a file offset and a
/// line number are. `usize` is at most 64 bits wide on every target this
/// crate builds for, so this refuses only on a wider one, and it refuses
/// rather than substitute a number the count is not.
///
/// # Errors
/// `io::Error` when `count` does not fit a `u64`.
pub(crate) fn u64_count(count: usize) -> io::Result<u64> {
    u64::try_from(count).map_err(io::Error::other)
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::super::*;
    use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};
    use std::fs;

    fn draft(kind: EventKind, t: u64) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(t),
            who: "city".to_string(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        }
    }

    /// The ledger directory holds segments and may hold other files; only a
    /// file named like a segment is read or repaired as one.
    #[test]
    fn a_file_not_named_like_a_segment_is_neither_read_nor_repaired() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger
            .append_all(vec![draft(EventKind::CityInitialized, 0)])
            .unwrap();
        let lines = ledger.read_raw_lines().unwrap();
        drop(ledger);
        let strays = [
            ("notes.txt", b"not a line".as_slice()),
            ("other.jsonl", b"{\"torn"),
        ];
        for (name, bytes) in strays {
            fs::write(dir.path().join(name), bytes).unwrap();
        }

        let (reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(1)).unwrap();
        let kept: Vec<Vec<u8>> = strays
            .iter()
            .map(|(name, _)| fs::read(dir.path().join(name)).unwrap())
            .collect();
        assert_eq!(
            (
                report.recovered.is_none(),
                reopened.read_raw_lines().unwrap(),
                kept
            ),
            (
                true,
                lines,
                strays.iter().map(|(_, bytes)| bytes.to_vec()).collect()
            )
        );
    }
}
