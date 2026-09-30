// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The chain state entering the last segment, read off the last line of
//! the segment before it, and that line alone.

use std::path::{Path, PathBuf};

use kernel::ledger::chain_hash;
use kernel::{AxCode, AxError, GENESIS_PREV, Seq};

use crate::error::{StorageError, io_err};

use super::ledger::{JsonlLedger, PriorSegment, TailBoundary, segment_first_seq};
use super::verify::{LineCheck, LineFault};
use crate::vfs::Vfs;

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
        let len = self
            .vfs
            .size(prior)
            .map_err(io_err("measure segment", prior))?;
        let count = line_count(prior, last);
        let last_line =
            last_line_of(self.vfs.as_ref(), prior, len).map_err(io_err("read segment", prior))?;
        let Some(last_line) = last_line else {
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
        let judged = LineCheck::judge(&last_line).map_err(|fault| refusal(prior, count, fault))?;
        let next = judged
            .seq
            .next()
            .map_err(|source| StorageError::Draft { source })?;
        Ok(TailBoundary {
            prev: chain_hash(&last_line),
            next_seq: next,
            prior: Some(PriorSegment {
                path: prior.clone(),
                len,
            }),
        })
    }
}

/// How far back from a segment's end the first read reaches for its last
/// line; the window doubles until the line's start is in it, so a line of
/// any length costs at most twice its own bytes and a segment's length
/// costs nothing (storage-SPEC 8-1).
const BOUNDARY_WINDOW: u64 = 16 * 1024;

/// The last complete line of the segment at `path`, `len` bytes long,
/// without its `\n`, read from the end; `None` when the segment holds no
/// complete line.
fn last_line_of(vfs: &dyn Vfs, path: &Path, len: u64) -> std::io::Result<Option<Vec<u8>>> {
    let mut window = BOUNDARY_WINDOW;
    loop {
        let from = len.saturating_sub(window);
        let tail = vfs.read_at(path, from, len.saturating_sub(from))?;
        let reached_start = from == 0;
        let body = tail
            .iter()
            .rposition(|byte| *byte == b'\n')
            .and_then(|end| tail.get(..end));
        match (body, reached_start) {
            (Some(body), _) => match body.iter().rposition(|byte| *byte == b'\n') {
                Some(start) => {
                    return Ok(Some(
                        body.get(start.saturating_add(1)..)
                            .unwrap_or_default()
                            .to_vec(),
                    ));
                }
                None if reached_start => return Ok(Some(body.to_vec())),
                None => {}
            },
            (None, true) => return Ok(None),
            (None, false) => {}
        }
        window = window.saturating_mul(2);
    }
}

/// Which line of `prior` its last line is, read off the two segments'
/// names rather than by counting: each name carries the seq of its first
/// line and every line carries the next seq. Zero when a name does not
/// say, which only a message ever reads.
fn line_count(prior: &Path, last: &Path) -> u64 {
    let first = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(segment_first_seq)
    };
    match (first(prior), first(last)) {
        (Some(prior), Some(last)) => last.value().saturating_sub(prior.value()),
        (None, _) | (_, None) => 0,
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
