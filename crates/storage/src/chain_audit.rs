// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole chain proved from genesis, one segment at a time, reusing
//! what an earlier strict walk proved; and the verdict the writer waits
//! for or halts on (storage-SPEC 8-30). The proof is a query; setting the
//! verdict is the caller's command.

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use kernel::{AxError, B3Hash};

use crate::error::{StorageError, io_err};
use crate::jsonl::{JsonlLedger, LineCheck, segment_names};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, SegmentRecord, line_check_version};
use crate::vfs::Vfs;

/// What walking the whole chain found.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainAudit {
    /// Every complete line continues the chain.
    Whole { lines: u64 },
    /// The first line that did not, as the reason a person reads.
    Broken(AxError),
}

/// What one proof read and computed: counts rather than time, so a test
/// can hold them at two sizes of history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProofCount {
    /// Lines checked one by one through `LineCheck`.
    pub lines_checked: u64,
    /// Segments whose record stood in for checking their prefix.
    pub segments_by_digest: u64,
    pub bytes_read: u64,
    pub bytes_hashed: u64,
}

/// A proof's verdict, what it cost, and the first record it could not
/// write, which changes no verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct Proven {
    pub audit: ChainAudit,
    pub counted: ProofCount,
    pub unkept: Option<AxError>,
}

/// The verdict a writer shares with the proof of its history: once set it
/// stays, and it carries the reason every refused write repeats.
#[derive(Debug, Clone, Default)]
pub struct ChainHalt {
    verdict: Arc<(Mutex<Option<Verdict>>, Condvar)>,
    before: Before,
}

#[derive(Debug, Clone)]
enum Verdict {
    Whole,
    Broken(AxError),
}

/// What the writer does before there is a verdict.
#[derive(Debug, Clone, Copy, Default)]
enum Before {
    /// The history was proved when the writer opened (`resume`, `fork`,
    /// a one-shot command), so nothing waits for a background proof.
    #[default]
    Admit,
    /// A served city opened from its snapshots: nothing is written after
    /// history nobody has walked yet.
    Refuse,
}

impl ChainHalt {
    /// A halt that refuses every append until a verdict is set.
    pub fn awaiting_proof() -> ChainHalt {
        ChainHalt {
            verdict: Arc::default(),
            before: Before::Admit,
        }
    }

    /// The chain is whole. The first verdict wins.
    pub fn prove(&self) {
        self.settle(Verdict::Whole);
    }

    /// Stop taking new work. The first verdict wins; a later reason is
    /// dropped, because the chain broke where the first proof said.
    pub fn trip(&self, reason: AxError) {
        self.settle(Verdict::Broken(reason));
    }

    /// Why new work is refused, when the chain is broken.
    pub fn reason(&self) -> Option<AxError> {
        match &*self.held() {
            Some(Verdict::Broken(reason)) => Some(reason.clone()),
            Some(Verdict::Whole) | None => None,
        }
    }

    /// Whether a proof found the chain whole.
    pub fn proved(&self) -> bool {
        matches!(&*self.held(), Some(Verdict::Whole))
    }

    /// Blocks until there is a verdict; returns at once for a halt that
    /// admits before one.
    pub fn await_verdict(&self) {
        match self.before {
            Before::Admit => {}
            Before::Refuse => {
                let (lock, settled) = &*self.verdict;
                let guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
                drop(
                    settled
                        .wait_while(guard, |verdict| verdict.is_none())
                        .unwrap_or_else(PoisonError::into_inner),
                );
            }
        }
    }

    pub(crate) fn admit(&self) -> Result<(), StorageError> {
        match (&*self.held(), self.before) {
            (Some(Verdict::Whole), _) | (None, Before::Admit) => Ok(()),
            (Some(Verdict::Broken(reason)), _) => Err(StorageError::ChainHalted {
                source: reason.clone(),
            }),
            (None, Before::Refuse) => Err(StorageError::Unproven),
        }
    }

    fn settle(&self, verdict: Verdict) {
        let (lock, settled) = &*self.verdict;
        let mut held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if held.is_none() {
            *held = Some(verdict);
            settled.notify_all();
        }
    }

    /// The verdict as it stands. A panic while it was held cannot leave it
    /// half-written: it is one assignment of a whole value.
    fn held(&self) -> MutexGuard<'_, Option<Verdict>> {
        self.verdict
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl JsonlLedger {
    /// Share `halt` with this writer: every later `append_all` asks it
    /// before framing a line.
    pub fn halt_on(&mut self, halt: ChainHalt) {
        self.halt = halt;
    }

    /// Blocks until the proof this writer waits for has a verdict.
    pub fn await_verdict(&self) {
        self.halt.await_verdict();
    }
}

/// Walk every complete line of the ledger in `dir` from genesis, each
/// through the one per-line check, reading no record and writing none.
/// Read-only; holds one segment at a time.
///
/// # Errors
/// `StorageError::Io` when a segment cannot be listed or read.
pub fn audit_chain(dir: &Path) -> Result<ChainAudit, StorageError> {
    walk(dir, None).map(|proven| proven.audit)
}

/// Prove the ledger in `dir` from genesis, taking a segment's record in
/// place of checking its prefix line by line when the record's rule
/// version, entry state and digest all match, and writing a record for
/// every segment proved when `records` may be written.
///
/// # Errors
/// `StorageError::Io` when a segment cannot be listed or read.
pub fn prove_chain(dir: &Path, records: &ProofRecords) -> Result<Proven, StorageError> {
    walk(dir, Some(records))
}

/// The one walk behind both faces.
fn walk(dir: &Path, records: Option<&ProofRecords>) -> Result<Proven, StorageError> {
    let vfs = RealFs::new();
    let version = line_check_version();
    let mut walked = Walked {
        check: LineCheck::at_genesis(),
        lines: 0,
        counted: ProofCount::default(),
        unkept: None,
    };
    for name in segment_names(&vfs, dir)? {
        let path = dir.join(&name);
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a segment to prove", &path))?;
        let known: Option<SegmentRecord> = records.and(None);
        let proved = match walked.segment(&bytes, known, version) {
            Ok(proved) => proved,
            Err(broken) => {
                return Ok(Proven {
                    audit: ChainAudit::Broken(broken),
                    counted: walked.counted,
                    unkept: walked.unkept,
                });
            }
        };
        if let Some(records) = records {
            let kept = records.keep(&name, &proved).map_err(StorageError::into_ax);
            walked.unkept = walked.unkept.take().or(kept.err());
        }
    }
    Ok(Proven {
        audit: ChainAudit::Whole {
            lines: walked.lines,
        },
        counted: walked.counted,
        unkept: walked.unkept,
    })
}

/// The chain state and the counts, between two segments.
struct Walked {
    check: LineCheck,
    lines: u64,
    counted: ProofCount,
    unkept: Option<AxError>,
}

impl Walked {
    /// Prove one segment's complete lines and answer the record of what
    /// was proved. The bytes checked and the bytes hashed are the same
    /// read (storage-SPEC 8-30).
    fn segment(
        &mut self,
        bytes: &[u8],
        known: Option<SegmentRecord>,
        version: u32,
    ) -> Result<SegmentRecord, AxError> {
        self.counted.bytes_read = self.counted.bytes_read.saturating_add(len_of(bytes));
        let entry = self.check;
        let complete = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at.saturating_add(1));
        let mut hasher = blake3::Hasher::new();
        let mut in_segment: u64 = 0;
        let from = match known.and_then(|record| self.reuse(bytes, &record, version, &mut hasher)) {
            Some((from, lines)) => {
                in_segment = lines;
                from
            }
            None => {
                hasher.reset();
                0
            }
        };
        let rest = bytes.get(from..complete).unwrap_or_default();
        hasher.update(rest);
        self.counted.bytes_hashed = self.counted.bytes_hashed.saturating_add(len_of(rest));
        for line in rest.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            self.lines = self.lines.saturating_add(1);
            in_segment = in_segment.saturating_add(1);
            self.counted.lines_checked = self.counted.lines_checked.saturating_add(1);
            self.check
                .advance(line)
                .map_err(|fault| fault.into_ax(self.lines))?;
        }
        Ok(SegmentRecord {
            version,
            len: len_of(bytes.get(..complete).unwrap_or_default()),
            lines: in_segment,
            entry,
            exit: self.check,
            digest: B3Hash::from_bytes(*hasher.finalize().as_bytes()),
        })
    }

    /// Takes `record` in place of checking the prefix it names when its
    /// rule version, its entry state and the digest of that prefix all
    /// match, answering where checking goes on and how many lines the
    /// prefix held; `hasher` has then hashed the prefix. `None` leaves
    /// the chain state where it was.
    fn reuse(
        &mut self,
        bytes: &[u8],
        record: &SegmentRecord,
        version: u32,
        hasher: &mut blake3::Hasher,
    ) -> Option<(usize, u64)> {
        if record.version != version || record.entry != self.check {
            return None;
        }
        let prefix = usize::try_from(record.len)
            .ok()
            .and_then(|len| bytes.get(..len))?;
        hasher.update(prefix);
        self.counted.bytes_hashed = self.counted.bytes_hashed.saturating_add(len_of(prefix));
        if B3Hash::from_bytes(*hasher.finalize().as_bytes()) != record.digest {
            return None;
        }
        self.lines = self.lines.saturating_add(record.lines);
        self.check = record.exit;
        self.counted.segments_by_digest = self.counted.segments_by_digest.saturating_add(1);
        Some((prefix.len(), record.lines))
    }
}

fn len_of(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
