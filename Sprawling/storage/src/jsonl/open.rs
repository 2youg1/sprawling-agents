// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, GENESIS_PREV, Seq, TimeMs};

use crate::chain_audit::ProofCount;
use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, TailStart};
use crate::vfs::Vfs;

use super::ledger::{
    JsonlLedger, OpenReport, SEGMENT_PREALLOCATION, SEGMENT_ROLL_BYTES, TailTruncation, WriterLock,
    is_segment, segment_file_name, u64_count,
};
use super::verify::{LineCheck, LineFault};

impl JsonlLedger {
    pub fn open(dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_locked(dir, now, TailProof::Strict)
    }

    pub fn open_reusing(
        dir: &Path,
        now: TimeMs,
        records: &ProofRecords,
    ) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_locked(dir, now, TailProof::Records(records))
    }

    fn open_locked(
        dir: &Path,
        now: TimeMs,
        proof: TailProof<'_>,
    ) -> Result<(Self, OpenReport), StorageError> {
        let lock = WriterLock::take(dir)?;
        let (mut ledger, report) =
            JsonlLedger::open_through(Box::new(RealFs::new()), dir, now, proof)?;
        ledger.lock = Some(lock);
        Ok((ledger, report))
    }

    #[cfg(any(test, feature = "fault"))]
    pub fn open_faulty(
        fs: crate::fault_fs::FaultFs,
        dir: &Path,
        now: TimeMs,
    ) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_with(Box::new(fs), dir, now)
    }

    #[cfg(any(test, feature = "fault"))]
    pub(crate) fn open_with(
        vfs: Box<dyn Vfs>,
        dir: &Path,
        now: TimeMs,
    ) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_through(vfs, dir, now, TailProof::Strict)
    }

    fn open_through(
        vfs: Box<dyn Vfs>,
        dir: &Path,
        now: TimeMs,
        proof: TailProof<'_>,
    ) -> Result<(Self, OpenReport), StorageError> {
        let (ledger, segments) = JsonlLedger::unopened(vfs, dir)?;
        ledger.resume(&segments, now, proof)
    }

    fn unopened(mut vfs: Box<dyn Vfs>, dir: &Path) -> Result<(Self, Vec<PathBuf>), StorageError> {
        vfs.create_dir_all(dir)
            .map_err(io_err("create ledger dir", dir))?;
        let segments: Vec<PathBuf> = vfs
            .list(dir)
            .map_err(io_err("list ledger dir", dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();

        let ledger = JsonlLedger {
            vfs,
            dir: dir.to_path_buf(),
            seg_path: dir.join(segment_file_name(Seq::FIRST)),
            seg_len: 0,
            next_seq: Seq::FIRST,
            prev: GENESIS_PREV,
            roll_bytes: SEGMENT_ROLL_BYTES,
            preallocation: SEGMENT_PREALLOCATION,
            barrier: super::barrier::Barrier::Whole,

            sessions: crate::sessions::Sessions::for_ledger(dir),
            observer: None,
            lock: None,
            halt: crate::chain_audit::ChainHalt::default(),
            pending_unwind: None,
        };
        Ok((ledger, segments))
    }

    fn resume(
        mut self,
        segments: &[PathBuf],
        now: TimeMs,
        proof: TailProof<'_>,
    ) -> Result<(Self, OpenReport), StorageError> {
        let Some(last) = segments.last() else {
            return Ok((
                self,
                OpenReport {
                    recovered: None,
                    counted: ProofCount::default(),
                },
            ));
        };
        self.probe_version(segments)?;
        let (dropped, counted) = self.recover_tail(segments, last, proof)?;
        let recovered = if dropped > 0 {
            if self.next_seq != Seq::FIRST {
                self.append_log_truncated(now, dropped)?;
            }
            Some(TailTruncation {
                dropped_bytes: dropped,
            })
        } else {
            None
        };
        Ok((self, OpenReport { recovered, counted }))
    }

    fn recover_tail(
        &mut self,
        segments: &[PathBuf],
        last: &Path,
        proof: TailProof<'_>,
    ) -> Result<(u64, ProofCount), StorageError> {
        let boundary = self.boundary(segments, last)?;
        let prior = boundary.prior;
        let start = proof
            .start(
                self.vfs.as_ref(),
                last,
                LineCheck::after(boundary.prev, boundary.next_seq),
            )
            .map_err(io_err("read segment", last))?;
        let mut check = start.check;
        let mut counted = start.counted;
        let from = u64_count(start.from).map_err(io_err("measure segment", last))?;
        let mut valid_len = from;
        let mut at = start.lines;
        let mut damaged = None;
        let end = super::reading::scan(self.vfs.as_ref(), last, from, |line, end| {
            at = at.saturating_add(1);
            if let Some(first_fault) = damaged {
                if LineCheck::carries_envelope(line) {
                    return Err(chain_refusal(last, first_fault));
                }
                return Ok(());
            }
            counted.lines_checked = counted.lines_checked.saturating_add(1);
            let fault = match check.advance(line) {
                Ok(_) => {
                    valid_len = end;
                    return Ok(());
                }
                Err(fault) => fault,
            };
            let source = match fault {
                LineFault::VersionAhead(v) => {
                    return Err(StorageError::VersionAhead {
                        path: last.to_path_buf(),
                        v,
                    });
                }
                LineFault::NotAVersion(v) => unversioned(v),
                LineFault::NotALine(_) => {
                    if LineCheck::carries_envelope(line) {
                        return Err(chain_refusal(last, at));
                    }
                    damaged = Some(at);
                    return Ok(());
                }
                LineFault::ChainBreak | LineFault::SeqGap { .. } => {
                    return Err(chain_refusal(last, at));
                }
                other @ (LineFault::UnknownKind(_)
                | LineFault::NotCanonical(_)
                | LineFault::SeqExhausted(_)) => other.into_ax(at),
            };
            Err(StorageError::Envelope {
                path: last.to_path_buf(),
                line: at,
                source,
            })
        })?;
        counted.bytes_read = counted
            .bytes_read
            .saturating_add(end.total.saturating_sub(from));
        let total = end.total;
        let run_prev = check.prev();
        let run_seq = check.expected();

        let dropped = end.filled.saturating_sub(valid_len);
        let keep = valid_len;

        if self.preallocation.length_after_open(keep, dropped, total) < total {
            if valid_len == 0 {
                self.vfs
                    .truncate(last, 0)
                    .map_err(io_err("truncate segment", last))?;
                self.vfs
                    .sync_data(last)
                    .map_err(io_err("sync segment", last))?;
                self.vfs
                    .remove_file(last)
                    .map_err(io_err("remove empty segment", last))?;
                self.vfs
                    .sync_dir(&self.dir.clone())
                    .map_err(io_err("sync ledger dir", &self.dir.clone()))?;
                match prior {
                    Some(segment) => {
                        self.seg_path = segment.path;
                        self.seg_len = segment.len;
                    }
                    None => {
                        self.seg_path = self.dir.join(segment_file_name(Seq::FIRST));
                        self.seg_len = 0;
                    }
                }
            } else {
                self.vfs
                    .truncate(last, keep)
                    .map_err(io_err("truncate segment", last))?;
                self.vfs
                    .sync_data(last)
                    .map_err(io_err("sync segment", last))?;
                self.seg_path = last.to_path_buf();
                self.seg_len = keep;
            }
        } else {
            self.seg_path = last.to_path_buf();
            self.seg_len = keep;
        }
        self.prev = run_prev;
        self.next_seq = run_seq;
        Ok((dropped, counted))
    }
}

#[derive(Clone, Copy)]
enum TailProof<'a> {
    Strict,

    Records(&'a ProofRecords),
}

impl TailProof<'_> {
    fn start(self, vfs: &dyn Vfs, last: &Path, entry: LineCheck) -> std::io::Result<TailStart> {
        match self {
            TailProof::Strict => Ok(TailStart::strict(entry)),
            TailProof::Records(records) => records.tail_start(vfs, last, entry),
        }
    }
}

fn chain_refusal(last: &Path, line: u64) -> StorageError {
    StorageError::Envelope {
        path: last.to_path_buf(), line,
        source: AxError::failure(AxCode::InvalidArgs, "verify chain", "a line does not continue the chain")
            .with_recovery("run `sprawling replay <ledger-dir>` to see the first line that breaks, then restore that segment from its checkpoint commit"),
    }
}

pub(super) fn unversioned(v: u64) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a ledger line's version",
        format!("v{v} is below the first version any build wrote"),
    )
    .with_recovery(
        "restore this segment from its checkpoint commit: the ledger version starts at 1 \
         and this line declares less",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod preallocated;

#[cfg(test)]
impl JsonlLedger {
    pub(crate) fn open_preallocated(
        fs: crate::fault_fs::FaultFs,
        dir: &Path,
        now: TimeMs,
    ) -> Result<(Self, OpenReport), StorageError> {
        let (mut ledger, segments) = JsonlLedger::unopened(Box::new(fs), dir)?;
        ledger.preallocation = super::ledger::SegmentPreallocation::ToRollSize;
        ledger.resume(&segments, now, TailProof::Strict)
    }
}
