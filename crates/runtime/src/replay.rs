// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Offline replay: re-verifies a Ledger without re-executing anything.
//! The second and last minting point for EventRef.
//!
//! Refusals are fail-closed and name the 1-based line: a higher `v` and
//! an unknown kind without `ig:true` speak direction
//! (`E_LOG_VERSION_UNSUPPORTED`, written by a newer sprawling); a broken
//! chain, a seq gap or non-canonical bytes are storage integrity
//! (`E_CAS_CORRUPT`). Lines with `ig:true` and an unknown kind skip the
//! typed parse but still count in the chain — the chain covers raw bytes,
//! not meanings.

use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash, EventRecord, EventRef, Seq};
use memory::{CheckedLine, LineCheck};

/// One verified line: a typed record with its ref echo, or an explicitly
/// ignorable line from a future vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifiedLine {
    Known { record: EventRecord, echo: EventRef },
    IgnoredUnknown { seq: Seq },
}

/// A verified sequence: raw bytes plus their verified reading. Forking
/// consumes this type, never raw lines — a fork of an unverified
/// sequence is unrepresentable (runtime-SPEC 8.5).
#[derive(Debug)]
pub struct VerifiedLedger {
    raw: Vec<Vec<u8>>,
    verified: Vec<VerifiedLine>,
}

impl VerifiedLedger {
    pub fn raw_lines(&self) -> &[Vec<u8>] {
        &self.raw
    }

    pub fn lines(&self) -> &[VerifiedLine] {
        &self.verified
    }

    pub fn tail_seq(&self) -> Option<Seq> {
        let count = u64::try_from(self.verified.len()).unwrap_or(u64::MAX);
        count.checked_sub(1).map(Seq::new)
    }
}

/// A2: offline chain verification over raw lines, through the one
/// per-line check `memory::LineCheck` owns.
pub fn verify_lines(lines: Vec<Vec<u8>>) -> Result<VerifiedLedger, AxError> {
    let mut check = LineCheck::at_genesis();
    let verified = lines
        .iter()
        .zip(1u64..)
        .map(|(raw, line_no)| {
            check
                .advance(raw)
                .map(|checked| match checked {
                    CheckedLine::Known(record) => VerifiedLine::Known {
                        echo: record.to_ref(),
                        record,
                    },
                    CheckedLine::IgnoredUnknown(seq) => VerifiedLine::IgnoredUnknown { seq },
                })
                .map_err(|fault| fault.into_ax(line_no))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(VerifiedLedger {
        raw: lines,
        verified,
    })
}

/// A2 over a durable ledger directory; strictly read-only.
///
/// A directory holding no segment yields an empty `VerifiedLedger`, the
/// same as a ledger holding no events. That is deliberate here: every
/// caller of this function computes the path from a city root it already
/// holds, and a city that has been opened but never written to has a
/// ledger directory and no segment in it. Telling the two apart is the
/// job of whoever took the path from a person - `sprawling replay` does
/// it with `memory::ledger_segments_at` (sprawling-SPEC section 12).
pub fn verify_ledger_dir(dir: &Path) -> Result<VerifiedLedger, AxError> {
    let lines = memory::read_raw_lines_at(dir).map_err(memory::MemoryError::into_ax)?;
    verify_lines(lines)
}

/// A2 folded instead of kept: each record the per-line check reads is
/// lent to `each` and dropped, so what stays resident is one segment's
/// bytes and one record rather than every raw line and every record.
///
/// # Errors
/// The first line that does not verify, named as `verify_ledger_dir`
/// names it, or the first error `each` returns. The records before that
/// line were already shown to `each`, so the caller discards everything
/// it folded.
pub fn fold_ledger_dir(
    dir: &Path,
    mut each: impl FnMut(&EventRecord) -> Result<(), AxError>,
) -> Result<(), AxError> {
    for line in verify_ledger_dir(dir)?.lines() {
        if let VerifiedLine::Known { record, .. } = line {
            each(record)?;
        }
    }
    Ok(())
}

/// A15: recompute the four segment hashes from a `prompt_assembled`
/// payload plus the same source documents, and check them against the
/// recorded ones. The concatenation rule (join separator, truncation
/// marker) is reused from `prefix` — one authority, no second copy.
pub fn rebuild_prefix(
    data: &serde_json::Value,
    resolver: &dyn Fn(&Address) -> Option<Vec<u8>>,
) -> Result<[B3Hash; 4], AxError> {
    let segments = data
        .get("segments")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "rebuild prefix",
                "payload has no segments",
            )
            .with_recovery(
                "replay a run whose `prompt_assembled` line carries `segments`; a \
                 hand-written line cannot be rebuilt",
            )
        })?;
    if segments.len() != 4 {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "rebuild prefix",
            format!("expected 4 segments, found {}", segments.len()),
        )
        .with_recovery(
            "replay a run recorded by this build: a prefix is four segments, city, \
             building, resident and run",
        ));
    }
    let mut hashes = Vec::with_capacity(4);
    for segment in segments {
        let slot = segment
            .get("slot")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        let sources = segment
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "rebuild prefix",
                    format!("{slot}: payload lacks source notes (hand-assembled prefix?)"),
                )
                .with_recovery(format!(
                    "replay a run whose {slot} segment lists the documents it was \
                     built from; a prefix assembled by hand names none"
                ))
            })?;
        let mut text = String::new();
        for source in sources {
            let addr_raw = source
                .get("addr")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AxError::failure(AxCode::InvalidArgs, "rebuild prefix", "source without addr")
                        .with_recovery(
                            "replay a run whose every source note carries `addr`; \
                             without it the document cannot be found again",
                        )
                })?;
            let addr = Address::parse(addr_raw)?;
            let bytes = resolver(&addr).ok_or_else(|| {
                AxError::failure(AxCode::PathNotFound, "rebuild prefix", addr_raw)
                    .with_recovery("supply the source document at its recorded address")
            })?;
            let body = std::str::from_utf8(&bytes).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "rebuild prefix",
                    format!("{addr_raw}: not utf-8"),
                )
                .with_recovery(format!(
                    "restore {addr_raw} to the UTF-8 text it held when the run \
                     recorded it; the bytes on disk today are something else"
                ))
            })?;
            let kept = usize::try_from(
                source
                    .get("kept")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            )
            .map_err(|_| {
                AxError::failure(AxCode::InvalidArgs, "rebuild prefix", "kept exceeds usize")
                    .with_recovery(
                        "replay this run on a 64-bit machine: the recorded kept span \
                         is longer than this one can address",
                    )
            })?;
            let marker = source
                .get("marker")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let dropped = source
                .get("dropped")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let piece = body.get(..kept).ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "rebuild prefix",
                    format!("{addr_raw}: kept span exceeds the document"),
                )
                .with_recovery(format!(
                    "restore {addr_raw} to the text it held when the run recorded it; \
                     the file on disk today is shorter than the span the run kept"
                ))
            })?;
            if !text.is_empty() {
                text.push_str(crate::prefix::DOC_JOIN);
            }
            text.push_str(piece);
            if marker {
                text.push_str(&crate::elision::marker(kernel::ByteLen::new(dropped)));
            }
        }
        let rebuilt = B3Hash::digest(text.as_bytes());
        if let Some(recorded) = segment.get("hash").and_then(serde_json::Value::as_str)
            && recorded != rebuilt.to_string()
        {
            return Err(AxError::failure(
                AxCode::CasCorrupt,
                "rebuild prefix",
                format!("{slot}: rebuilt hash differs from the recorded one"),
            )
            .with_recovery("the source documents no longer match the recorded assembly"));
        }
        hashes.push(rebuilt);
    }
    let four: [B3Hash; 4] = match hashes.try_into() {
        Ok(array) => array,
        Err(_) => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "rebuild prefix",
                "segment count drifted during rebuild",
            )
            .with_recovery(
                "report this against runtime::replay: four segments went into the \
                 rebuild and a different number came out",
            ));
        }
    };
    Ok(four)
}

/// Crash recovery - detecting a call with no outcome, and closing it -
/// is the other half of replay and has its own file.
mod resume;

pub use resume::{dangling_tool_calls, outcome_unknown_draft};

/// The cache shape a replay rebuilds from two records and then judges
/// for itself.
mod shape;

pub use shape::rebuild_shape;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
