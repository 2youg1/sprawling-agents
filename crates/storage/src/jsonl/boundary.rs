// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The chain state entering the last segment, read off the segment before it.

use std::path::{Path, PathBuf};

use kernel::ledger::chain_hash;
use kernel::{AxCode, AxError, GENESIS_PREV, Seq};

use crate::error::{StorageError, io_err};

use super::ledger::{JsonlLedger, PriorSegment, TailBoundary, complete_lines};
use super::verify::{LineCheck, LineFault};

impl JsonlLedger {
    /// Boundary state entering the last segment: chain root, or the
    /// previous segment's last line, judged by the same rule as every
    /// other line (`LineCheck::judge`), so an ignorable line of a newer
    /// kind closes a segment as lawfully as it sits inside one.
    pub(super) fn boundary(
        &mut self,
        segments: &[PathBuf],
        last: &Path,
    ) -> Result<TailBoundary, StorageError> {
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
            return Err(StorageError::Envelope {
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
        let judged = LineCheck::judge(last_line).map_err(|fault| refusal(prior, count, fault))?;
        let next = judged
            .seq
            .next()
            .map_err(|source| StorageError::Draft { source })?;
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
}

/// How open refuses the prior segment's last line: a newer writer by
/// its version, anything else as damage at that line, the same way
/// `recover_tail` refuses a line of the last segment.
fn refusal(prior: &Path, line: u64, fault: LineFault) -> StorageError {
    match fault {
        LineFault::VersionAhead(v) => StorageError::VersionAhead {
            path: prior.to_path_buf(),
            v,
        },
        other @ (LineFault::NotALine(_)
        | LineFault::NotAVersion(_)
        | LineFault::ChainBreak
        | LineFault::SeqGap { .. }
        | LineFault::UnknownKind(_)
        | LineFault::NotCanonical(_)
        | LineFault::SeqExhausted(_)) => StorageError::Envelope {
            path: prior.to_path_buf(),
            line,
            source: other.into_ax(line),
        },
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
