// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger opening: version probes and tail recovery.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, GENESIS_PREV, Seq, TimeMs};

use crate::chain_audit::ProofCount;
use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::verified_prefix::{ProofRecords, TailStart};
use crate::vfs::Vfs;

use super::ledger::{
    JsonlLedger, OpenReport, SEGMENT_PREALLOCATION, SEGMENT_ROLL_BYTES, TailTruncation, WriterLock,
    complete_lines, is_segment, records_end, segment_file_name, u64_count,
};
use super::verify::{LineCheck, LineFault};

impl JsonlLedger {
    /// Production entrance: std filesystem underneath, and the ledger
    /// directory's writer lock, taken before anything is read or
    /// repaired and held for as long as the returned ledger lives.
    ///
    /// # Errors
    /// `LedgerHeld` when another writer holds this ledger, in this
    /// process or another; otherwise whatever reading and repairing the
    /// segments reports.
    pub fn open(dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_locked(dir, now, TailProof::Strict)
    }

    /// [`JsonlLedger::open`], with the last segment's verified-prefix
    /// record in `records` standing in for checking its prefix line by
    /// line when the record holds (`crates/storage/spec/Jsonl.lean` §8-34). Records are read,
    /// never written: the proof that holds the writer lock writes them.
    ///
    /// # Errors
    /// Those of [`JsonlLedger::open`].
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

    /// The `fault` entrance: the same ledger, over the deterministic
    /// power-loss model, for a caller that needs one named write to fail.
    ///
    /// Takes the concrete adapter rather than the trait, so the `Vfs`
    /// seam stays inner (`crates/storage/spec/Jsonl.lean` §8-1) and no `pub trait` leaves this
    /// crate. What comes back is the same `JsonlLedger` production uses -
    /// a caller above this crate exercises its real code and loses only
    /// the write it named.
    ///
    /// # Errors
    /// Propagates whatever [`JsonlLedger::open`] would, and the power
    /// loss itself when the plan cuts during the open.
    #[cfg(any(test, feature = "fault"))]
    pub fn open_faulty(
        fs: crate::fault_fs::FaultFs,
        dir: &Path,
        now: TimeMs,
    ) -> Result<(Self, OpenReport), StorageError> {
        JsonlLedger::open_with(Box::new(fs), dir, now)
    }

    /// Injection point for the second Vfs adapter, checking the last
    /// segment line by line; `open_faulty` is its entrance.
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

    /// The ledger at the root of an empty chain, with the segments `dir`
    /// holds, before anything on them is read.
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
            // The projection exists only when this ledger is a city's:
            // it is what tells a writer handed the ledger directory
            // alone where the buildings' sessions lie. A store opened
            // directly has no buildings, and files nothing.
            sessions: crate::sessions::Sessions::for_ledger(dir),
            observer: None,
            lock: None,
            halt: crate::chain_audit::ChainHalt::default(),
            pending_unwind: None,
        };
        Ok((ledger, segments))
    }

    /// Reads the version and the tail of `segments` and puts the writer
    /// at the end of the surviving records.
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

    /// Tail-truncation recovery over the last segment, taking its
    /// verified prefix from `proof` when the record holds (`crates/storage/spec/Jsonl.lean`
    /// §8-34). Returns dropped bytes and what was read and checked; on
    /// return the ledger state points at the surviving tail.
    fn recover_tail(
        &mut self,
        segments: &[PathBuf],
        last: &Path,
        proof: TailProof<'_>,
    ) -> Result<(u64, ProofCount), StorageError> {
        let boundary = self.boundary(segments, last)?;
        let prior = boundary.prior;
        let file = self.vfs.read(last).map_err(io_err("read segment", last))?;
        // Zeros after the last record are preallocated space, not a tear
        // (`crates/storage/spec/Jsonl/Preallocate.lean`): the scan reads the
        // segment without them, and the writer resumes where they begin.
        #[cfg(test)]
        super::reading::measure(file.capacity(), file.len());
        let filled = records_end(&file);
        let total = u64_count(file.len()).map_err(io_err("measure segment", last))?;
        let mut bytes = file;
        bytes.truncate(filled);
        let start = proof.start(
            last,
            &bytes,
            LineCheck::after(boundary.prev, boundary.next_seq),
        );
        let mut check = start.check;
        let mut counted = start.counted;
        let (lines, _terminated_len) = complete_lines(bytes.get(start.from..).unwrap_or_default());

        let mut valid_len = start.from;
        for (index, line) in lines.iter().enumerate() {
            let at = u64_count(index)
                .map_err(io_err("number a segment line", last))?
                .saturating_add(1)
                .saturating_add(start.lines);
            counted.lines_checked = counted.lines_checked.saturating_add(1);
            let fault = match check.advance(line) {
                Ok(_) => {
                    valid_len = valid_len.saturating_add(line.len()).saturating_add(1);
                    continue;
                }
                Err(fault) => fault,
            };
            // What a version refuses, it refuses by name: a line this
            // build cannot read is not tail damage, and truncating it
            // would delete a newer build's history (`crates/storage/spec/Jsonl.lean` §8-1).
            // A tear only ever damages the tail and never leaves a record
            // behind, so a break is refused - not truncated - when the
            // breaking line carries an envelope (a fork: a second writer
            // continued the same prev) or intact records still follow it
            // (non-tail damage). Newline-bearing garbage is no record.
            // The breaking line itself counts: a record behind a run of
            // zeros still carries its envelope.
            let later_intact = || {
                lines
                    .iter()
                    .skip(index)
                    .any(|l| LineCheck::carries_envelope(l))
            };
            let source = match fault {
                LineFault::VersionAhead(v) => {
                    return Err(StorageError::VersionAhead {
                        path: last.to_path_buf(),
                        v,
                    });
                }
                LineFault::NotAVersion(v) => unversioned(v),
                LineFault::NotALine(_) if !later_intact() => break,
                LineFault::ChainBreak | LineFault::SeqGap { .. } | LineFault::NotALine(_) => {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "verify chain",
                        "a line does not continue the chain",
                    )
                    .with_recovery(
                        "run `sprawling replay <ledger-dir>` to see the first line \
                     that breaks, then restore that segment from its \
                     checkpoint commit",
                    )
                }
                other @ (LineFault::UnknownKind(_)
                | LineFault::NotCanonical(_)
                | LineFault::SeqExhausted(_)) => other.into_ax(at),
            };
            return Err(StorageError::Envelope {
                path: last.to_path_buf(),
                line: at,
                source,
            });
        }
        let run_prev = check.prev();
        let run_seq = check.expected();

        let measure = |count: usize| u64_count(count).map_err(io_err("measure segment", last));
        let dropped = measure(filled.saturating_sub(valid_len))?;
        let keep = measure(valid_len)?;

        if self.preallocation.length_after_open(keep, dropped, total) < total {
            if valid_len == 0 {
                // Whole last segment is torn. Remove it; the tail falls
                // back to the prior segment (or to an empty city when this
                // was the only one — the genesis append never returned Ok).
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

/// What tail recovery may take in place of checking the last segment's
/// prefix line by line.
#[derive(Clone, Copy)]
enum TailProof<'a> {
    /// Every line of the last segment is checked.
    Strict,
    /// The last segment's record in these, when it holds (`crates/storage/spec/Jsonl.lean` §8-34).
    Records(&'a ProofRecords),
}

impl TailProof<'_> {
    /// Where checking `bytes`, the last segment at `last`, begins when
    /// the chain state entering it is `entry`.
    fn start(self, last: &Path, bytes: &[u8], entry: LineCheck) -> TailStart {
        match (self, last.file_name().and_then(|name| name.to_str())) {
            (TailProof::Records(records), Some(name)) => records.tail_start(name, bytes, entry),
            (TailProof::Records(_), None) | (TailProof::Strict, _) => {
                TailStart::strict(bytes, entry)
            }
        }
    }
}

/// The refusal a line below the first ledger version earns.
///
/// One sentence for both readers, because "nobody ever wrote this" is
/// one fact whether the probe or the tail scan meets it first.
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
    /// The `fault` entrance with the preallocating arm, whatever
    /// [`SEGMENT_PREALLOCATION`] says, so its tests judge the arm the
    /// build does not select (storage D31).
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
