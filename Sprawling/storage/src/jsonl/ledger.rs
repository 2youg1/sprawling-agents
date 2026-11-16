// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::path::{Path, PathBuf};

use kernel::{B3Hash, EventRecord, Seq};

use crate::error::{StorageError, io_err};
use crate::vfs::Vfs;

pub(crate) const SEGMENT_ROLL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentPreallocation {
    Grow,

    ToRollSize,
}

impl SegmentPreallocation {
    pub(crate) fn length_after_open(self, keep: u64, dropped: u64, total: u64) -> u64 {
        match (self, dropped) {
            (SegmentPreallocation::ToRollSize, 0) => total,
            (SegmentPreallocation::ToRollSize | SegmentPreallocation::Grow, _) => keep,
        }
    }
}

pub(crate) const SEGMENT_PREALLOCATION: SegmentPreallocation = SegmentPreallocation::Grow;

pub(crate) fn records_end(segment: &[u8]) -> usize {
    segment
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |at| at.saturating_add(1))
}

pub struct OpenReport {
    pub recovered: Option<TailTruncation>,
    pub counted: crate::ProofCount,
}

pub struct TailTruncation {
    pub dropped_bytes: u64,
}

pub struct JsonlLedger {
    pub(crate) vfs: Box<dyn Vfs>,
    pub(crate) dir: PathBuf,
    pub(crate) seg_path: PathBuf,
    pub(crate) seg_len: u64,
    pub(crate) next_seq: Seq,
    pub(crate) prev: B3Hash,
    pub(crate) roll_bytes: u64,

    pub(crate) preallocation: SegmentPreallocation,

    pub(crate) barrier: super::barrier::Barrier,

    pub(crate) sessions: Option<crate::sessions::Sessions>,

    pub(crate) observer: Option<WriteObserver>,

    pub(crate) lock: Option<WriterLock>,

    pub(crate) halt: crate::chain_audit::ChainHalt,

    pub(crate) pending_unwind: Option<Vec<PathBuf>>,
}

pub(crate) struct WriterLock {
    _held: std::fs::File,
}

pub struct LedgerHold {
    _lock: WriterLock,
}

impl LedgerHold {
    pub fn take(dir: &Path) -> Result<LedgerHold, StorageError> {
        Ok(LedgerHold {
            _lock: WriterLock::take(dir)?,
        })
    }
}

impl WriterLock {
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

pub type WriteObserver = Box<dyn FnMut(&EventRecord) + Send>;

pub(crate) struct TailBoundary {
    pub(crate) prev: B3Hash,
    pub(crate) next_seq: Seq,
    pub(crate) prior: Option<PriorSegment>,
}

pub(crate) struct PriorSegment {
    pub(crate) path: PathBuf,
    pub(crate) len: u64,
}

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

pub(crate) fn segment_first_seq(name: &str) -> Option<Seq> {
    let digits = name
        .strip_prefix(SEGMENT_PREFIX)?
        .strip_suffix(SEGMENT_SUFFIX)?;
    digits.parse::<u64>().ok().map(Seq::new)
}

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

pub(crate) fn u64_count(count: usize) -> io::Result<u64> {
    u64::try_from(count).map_err(io::Error::other)
}

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
