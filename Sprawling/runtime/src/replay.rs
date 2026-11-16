// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash, EventRecord, EventRef, Seq};
use storage::{CheckedLine, LedgerIndex, LineCheck, Located};

#[derive(Debug, Clone, PartialEq)]
pub enum VerifiedLine {
    Known { record: EventRecord, echo: EventRef },
    IgnoredUnknown { seq: Seq },
}

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

pub fn verify_ledger_dir(dir: &Path) -> Result<VerifiedLedger, AxError> {
    let lines = storage::read_raw_lines_at(dir).map_err(storage::StorageError::into_ax)?;
    verify_lines(lines)
}

pub fn fold_ledger_dir(
    dir: &Path,
    mut each: impl FnMut(&EventRecord) -> Result<(), AxError>,
) -> Result<LedgerIndex, AxError> {
    let mut check = LineCheck::at_genesis();
    let mut line_no = 0u64;
    LedgerIndex::folding(dir, |raw| {
        line_no = line_no.saturating_add(1);
        match check.advance(raw).map_err(|fault| fault.into_ax(line_no))? {
            CheckedLine::Known(record) => {
                each(&record)?;
                Ok(Some(Located {
                    seq: record.seq(),
                    run: Some(record.run()),
                }))
            }
            CheckedLine::IgnoredUnknown(_) => Ok(None),
        }
    })
}

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

            let recorded = match source.get("hash") {
                None | Some(serde_json::Value::Null) => None,
                Some(value) => Some(value.as_str().ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "rebuild prefix",
                        format!("{slot}: {addr_raw}: the recorded hash is not a string"),
                    )
                    .with_recovery(
                        "replay a run whose source rows were written by sprawling; this \
                         row's hash was altered after it was recorded",
                    )
                })?),
            };
            if let Some(recorded) = recorded
                && recorded != B3Hash::digest(piece.as_bytes()).to_string()
            {
                return Err(AxError::failure(
                    AxCode::CasCorrupt,
                    "rebuild prefix",
                    format!("{slot}: {addr_raw}: the kept bytes differ from the ones recorded"),
                )
                .with_recovery(format!(
                    "restore {addr_raw} to the text it held when the run recorded it; the \
                     model was given those bytes, not the ones on disk today"
                )));
            }
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

mod resume;

pub use resume::{DanglingCalls, outcome_unknown_draft};

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
