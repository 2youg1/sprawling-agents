// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger opening: version probes, tail recovery, segment boundaries.

use std::path::{Path, PathBuf};

use kernel::consts_external::{LogVersion, readable_log_v};
use kernel::ledger::chain_hash;
use kernel::{AxCode, AxError, EventRecord, GENESIS_PREV, Seq, TimeMs};

use crate::error::{MemoryError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::ledger::{
    JsonlLedger, OpenReport, PriorSegment, SEGMENT_ROLL_BYTES, TailBoundary, TailTruncation,
    WriterLock, complete_lines, is_segment, segment_file_name,
};
use super::verify::{LineCheck, LineFault};

impl JsonlLedger {
    /// Production entrance: std filesystem underneath, and for a city's
    /// ledger the city's writer lock, taken before anything is read or
    /// repaired and held for as long as the returned ledger lives.
    ///
    /// # Errors
    /// `LedgerHeld` when another writer holds this city's ledger, in this
    /// process or another; otherwise whatever reading and repairing the
    /// segments reports.
    pub fn open(dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), MemoryError> {
        let lock = WriterLock::take(dir)?;
        let (mut ledger, report) = JsonlLedger::open_with(Box::new(RealFs::new()), dir, now)?;
        ledger.lock = lock;
        Ok((ledger, report))
    }

    /// The `fault` entrance: the same ledger, over the deterministic
    /// power-loss model, for a caller that needs one named write to fail.
    ///
    /// Takes the concrete adapter rather than the trait, so the `Vfs`
    /// seam stays inner (memory-SPEC 8-1) and no `pub trait` leaves this
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
    ) -> Result<(Self, OpenReport), MemoryError> {
        JsonlLedger::open_with(Box::new(fs), dir, now)
    }

    /// Injection point for the second Vfs adapter; [`JsonlLedger::open`]
    /// and `open_faulty` are its two entrances.
    pub(crate) fn open_with(
        mut vfs: Box<dyn Vfs>,
        dir: &Path,
        now: TimeMs,
    ) -> Result<(Self, OpenReport), MemoryError> {
        vfs.create_dir_all(dir)
            .map_err(io_err("create ledger dir", dir))?;
        let segments: Vec<PathBuf> = vfs
            .list(dir)
            .map_err(io_err("list ledger dir", dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();

        let mut ledger = JsonlLedger {
            vfs,
            dir: dir.to_path_buf(),
            seg_path: dir.join(segment_file_name(Seq::FIRST)),
            seg_len: 0,
            next_seq: Seq::FIRST,
            prev: GENESIS_PREV,
            roll_bytes: SEGMENT_ROLL_BYTES,
            // The projection exists only when this ledger is a city's:
            // it is what tells a writer handed the ledger directory
            // alone where the buildings' sessions lie. A store opened
            // directly has no buildings, and files nothing.
            sessions: crate::sessions::Sessions::for_ledger(dir),
            observer: None,
            lock: None,
        };

        let Some(last) = segments.last().cloned() else {
            return Ok((ledger, OpenReport { recovered: None }));
        };

        ledger.probe_version(&segments)?;
        let dropped = ledger.recover_tail(&segments, &last)?;
        let report = if dropped > 0 {
            if ledger.next_seq != Seq::FIRST {
                ledger.append_log_truncated(now, dropped)?;
            }
            OpenReport {
                recovered: Some(TailTruncation {
                    dropped_bytes: dropped,
                }),
            }
        } else {
            OpenReport { recovered: None }
        };
        Ok((ledger, report))
    }

    /// Direction-aware version refusal, before any repair or parse
    /// (never a partial read of a newer ledger).
    fn probe_version(&mut self, segments: &[PathBuf]) -> Result<(), MemoryError> {
        let Some(first) = segments.first() else {
            return Ok(());
        };
        let bytes = self
            .vfs
            .read(first)
            .map_err(io_err("read segment", first))?;
        let (lines, _) = complete_lines(&bytes);
        let Some(first_line) = lines.first() else {
            // Empty or torn-before-first-line segment: version unknowable;
            // tail recovery decides what remains.
            return Ok(());
        };
        // A mangled first line in a single-segment ledger is tail damage:
        // it carries no version information, and tail recovery owns it.
        // With more segments behind it the same damage is non-tail and
        // must refuse instead (memory-SPEC 8-1).
        let probed = serde_json::from_slice::<serde_json::Value>(first_line)
            .ok()
            .and_then(|value| value.get("v").and_then(serde_json::Value::as_u64));
        let v = match probed {
            Some(v) => v,
            None if segments.len() > 1 => {
                return Err(MemoryError::Envelope {
                    path: first.clone(),
                    line: 1,
                    source: AxError::failure(
                        AxCode::InvalidArgs,
                        "probe ledger version",
                        "first line is not a version-bearing record",
                    )
                    .with_recovery(
                        "restore this segment from its checkpoint commit, then run \
                         `sprawling replay <ledger-dir>`: line 1 of every segment carries `v`",
                    ),
                });
            }
            None => return Ok(()),
        };
        match readable_log_v(v) {
            // An older ledger opens: the history is append-only and the
            // lines an earlier build wrote are still its history.
            LogVersion::Current | LogVersion::Older => Ok(()),
            LogVersion::Ahead => Err(MemoryError::VersionAhead {
                path: first.clone(),
                v,
            }),
            LogVersion::NotAVersion => Err(MemoryError::Envelope {
                path: first.clone(),
                line: 1,
                source: unversioned(v),
            }),
        }
    }

    /// Boundary state entering the last segment: chain root, or the
    /// previous segment's verified last line.
    fn boundary(&mut self, segments: &[PathBuf], last: &Path) -> Result<TailBoundary, MemoryError> {
        let mut prior: Option<&PathBuf> = None;
        for seg in segments {
            if seg.as_path() == last {
                break;
            }
            prior = Some(seg);
        }
        let Some(prior) = prior else {
            return Ok(TailBoundary {
                prev: GENESIS_PREV,
                next_seq: Seq::FIRST,
                prior: None,
            });
        };
        let bytes = self
            .vfs
            .read(prior)
            .map_err(io_err("read segment", prior))?;
        let (lines, _) = complete_lines(&bytes);
        let count = u64::try_from(lines.len()).unwrap_or(u64::MAX);
        let Some(last_line) = lines.last() else {
            return Err(MemoryError::Envelope {
                path: prior.clone(),
                line: 0,
                source: AxError::failure(
                    AxCode::InvalidArgs,
                    "read prior segment",
                    "segment holds no complete line",
                )
                .with_recovery(
                    "restore this segment from its checkpoint commit, or move it aside if \
                     it was never written: a segment that exists holds at least one line",
                ),
            });
        };
        let record =
            EventRecord::parse_line(last_line).map_err(|source| MemoryError::Envelope {
                path: prior.clone(),
                line: count,
                source,
            })?;
        let next = record
            .seq()
            .next()
            .map_err(|source| MemoryError::Draft { source })?;
        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        Ok(TailBoundary {
            prev: chain_hash(last_line),
            next_seq: next,
            prior: Some(PriorSegment {
                path: prior.clone(),
                len,
            }),
        })
    }

    /// Tail-truncation recovery over the last segment. Returns dropped
    /// bytes; on return the ledger state points at the surviving tail.
    fn recover_tail(&mut self, segments: &[PathBuf], last: &Path) -> Result<u64, MemoryError> {
        let boundary = self.boundary(segments, last)?;
        let mut check = LineCheck::after(boundary.prev, boundary.next_seq);
        let prior = boundary.prior;
        let bytes = self.vfs.read(last).map_err(io_err("read segment", last))?;
        let (lines, _terminated_len) = complete_lines(&bytes);

        let mut valid_len = 0usize;
        for (index, line) in lines.iter().enumerate() {
            let at = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
            let fault = match check.advance(line) {
                Ok(_) => {
                    valid_len = valid_len.saturating_add(line.len()).saturating_add(1);
                    continue;
                }
                Err(fault) => fault,
            };
            // What a version refuses, it refuses by name: a line this
            // build cannot read is not tail damage, and truncating it
            // would delete a newer build's history (memory-SPEC 8-1).
            // A tear only ever damages the tail and never leaves a record
            // behind, so a break is refused - not truncated - when the
            // breaking line carries an envelope (a fork: a second writer
            // continued the same prev) or intact records still follow it
            // (non-tail damage). Newline-bearing garbage is no record.
            let later_intact = || {
                lines
                    .iter()
                    .skip(index.saturating_add(1))
                    .any(|l| LineCheck::carries_envelope(l))
            };
            let source = match fault {
                LineFault::VersionAhead(v) => {
                    return Err(MemoryError::VersionAhead {
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
            return Err(MemoryError::Envelope {
                path: last.to_path_buf(),
                line: at,
                source,
            });
        }
        let run_prev = check.prev();
        let run_seq = check.expected();

        let total = bytes.len();
        let dropped = u64::try_from(total.saturating_sub(valid_len)).unwrap_or(u64::MAX);

        if dropped > 0 {
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
                let keep = u64::try_from(valid_len).unwrap_or(u64::MAX);
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
            self.seg_len = u64::try_from(total).unwrap_or(u64::MAX);
        }
        self.prev = run_prev;
        self.next_seq = run_seq;
        Ok(dropped)
    }
}

/// The refusal a line below the first ledger version earns.
///
/// One sentence for both readers, because "nobody ever wrote this" is
/// one fact whether the probe or the tail scan meets it first.
fn unversioned(v: u64) -> AxError {
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
