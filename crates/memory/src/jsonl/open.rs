// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger opening: version probes, tail recovery, segment boundaries.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, EventRecord, GENESIS_PREV, Seq, TimeMs, chain_hash};

use crate::error::{MemoryError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::ledger::{
    JsonlLedger, OpenReport, PriorSegment, SEGMENT_ROLL_BYTES, TailBoundary, TailTruncation,
    complete_lines, is_segment, segment_file_name,
};

impl JsonlLedger {
    /// Production entrance: std filesystem underneath.
    pub fn open(dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), MemoryError> {
        JsonlLedger::open_with(Box::new(RealFs::new()), dir, now)
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
            observer: None,
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
                    ),
                });
            }
            None => return Ok(()),
        };
        let current = u64::from(kernel::consts_external::EVENT_LOG_V);
        if v > current {
            return Err(MemoryError::VersionAhead {
                path: first.clone(),
                v,
            });
        }
        if v < 1 {
            return Err(MemoryError::Envelope {
                path: first.clone(),
                line: 1,
                source: AxError::failure(AxCode::InvalidArgs, "probe ledger version", "v < 1"),
            });
        }
        Ok(())
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
                source: AxError::failure(AxCode::InvalidArgs, "read prior segment", "empty"),
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
        let mut run_prev = boundary.prev;
        let mut run_seq = boundary.next_seq;
        let prior = boundary.prior;
        let bytes = self.vfs.read(last).map_err(io_err("read segment", last))?;
        let (lines, _terminated_len) = complete_lines(&bytes);

        let mut valid_len = 0usize;
        for (index, line) in lines.iter().enumerate() {
            let parsed = EventRecord::parse_line(line);
            let ok = match &parsed {
                Ok(record) => {
                    record.prev() == run_prev
                        && record.seq() == run_seq
                        && record.v() == kernel::consts_external::EVENT_LOG_V
                }
                Err(_) => false,
            };
            if !ok {
                // A tear only ever damages the tail. A bad first line is
                // refused — not silently discarded — when it is parseable
                // with a wrong chain root (foreign or damaged history) or
                // when intact records still follow it (non-tail damage).
                // Newline-bearing garbage does not count as a record.
                if index == 0 {
                    let later_intact = lines
                        .iter()
                        .skip(1)
                        .any(|l| EventRecord::parse_line(l).is_ok());
                    if parsed.is_ok() || later_intact {
                        return Err(MemoryError::Envelope {
                            path: last.to_path_buf(),
                            line: 1,
                            source: AxError::failure(
                                AxCode::InvalidArgs,
                                "verify chain root",
                                "first line does not continue the chain",
                            ),
                        });
                    }
                }
                break;
            }
            run_prev = chain_hash(line);
            run_seq = run_seq
                .next()
                .map_err(|source| MemoryError::Draft { source })?;
            valid_len = valid_len.saturating_add(line.len()).saturating_add(1);
        }

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
