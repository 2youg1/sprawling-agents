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

use kernel::{
    Address, AxCode, AxError, B3Hash, EventDraft, EventKind, EventRecord, EventRef, GENESIS_PREV,
    RunId, Seq, TimeMs, chain_hash, consts_external::EVENT_LOG_V,
};
use serde::Deserialize;

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

/// The envelope probe: enough of any line to judge version, chain and
/// kind before committing to a typed parse. Unknown extra fields pass —
/// this shape must read lines from the future.
#[derive(Deserialize)]
struct Envelope {
    v: u32,
    seq: Seq,
    prev: B3Hash,
    kind: String,
    #[serde(default)]
    ig: bool,
}

fn corrupt(line_no: u64, violation: impl Into<String>) -> AxError {
    AxError::failure(
        AxCode::CasCorrupt,
        "verify ledger",
        format!("line {line_no}"),
    )
    .with_recovery(violation.into())
}

/// A2: offline chain verification over raw lines.
pub fn verify_lines(lines: Vec<Vec<u8>>) -> Result<VerifiedLedger, AxError> {
    let mut verified = Vec::with_capacity(lines.len());
    let mut prev = GENESIS_PREV;
    let mut expected = Seq::FIRST;
    let mut line_no: u64 = 0;

    for raw in &lines {
        line_no = line_no.saturating_add(1);

        let envelope: Envelope = serde_json::from_slice(raw)
            .map_err(|e| corrupt(line_no, format!("not a ledger line: {e}")))?;

        if envelope.v > EVENT_LOG_V {
            return Err(AxError::failure(
                AxCode::LogVersionUnsupported,
                "verify ledger",
                format!("line {line_no} carries v{}", envelope.v),
            )
            .with_recovery(
                "written by a newer sprawling; replay it with the version that wrote it",
            ));
        }
        if envelope.v != EVENT_LOG_V {
            return Err(corrupt(line_no, format!("impossible v{}", envelope.v)));
        }
        if envelope.prev != prev {
            return Err(corrupt(line_no, "prev does not hash the previous line"));
        }
        if envelope.seq != expected {
            return Err(corrupt(
                line_no,
                format!(
                    "seq {} where {} was expected",
                    envelope.seq.value(),
                    expected.value()
                ),
            ));
        }

        let known_kind: Option<EventKind> =
            serde_json::from_value(serde_json::Value::String(envelope.kind.clone())).ok();
        match known_kind {
            Some(_) => {
                let record = EventRecord::parse_line(raw)
                    .map_err(|e| corrupt(line_no, format!("typed parse failed: {e}")))?;
                let echo = record.canonical_line()?;
                if &echo != raw {
                    return Err(corrupt(line_no, "bytes are not writer-canonical"));
                }
                let minted = record.to_ref();
                verified.push(VerifiedLine::Known {
                    record,
                    echo: minted,
                });
            }
            None if envelope.ig => {
                verified.push(VerifiedLine::IgnoredUnknown { seq: envelope.seq });
            }
            None => {
                return Err(AxError::failure(
                    AxCode::LogVersionUnsupported,
                    "verify ledger",
                    format!("line {line_no} kind `{}`", envelope.kind),
                )
                .with_recovery(
                    "unknown kind without ig:true means a newer writer; \
                     use the sprawling that wrote it",
                ));
            }
        }

        prev = chain_hash(raw);
        expected = expected.next()?;
    }

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
        })?;
    if segments.len() != 4 {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "rebuild prefix",
            format!("expected 4 segments, found {}", segments.len()),
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
            })?;
        let mut text = String::new();
        for source in sources {
            let addr_raw = source
                .get("addr")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AxError::failure(AxCode::InvalidArgs, "rebuild prefix", "source without addr")
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
            })?;
            let kept = usize::try_from(
                source
                    .get("kept")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            )
            .map_err(|_| {
                AxError::failure(AxCode::InvalidArgs, "rebuild prefix", "kept exceeds usize")
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
            })?;
            if !text.is_empty() {
                text.push_str(crate::prefix::DOC_JOIN);
            }
            text.push_str(piece);
            if marker {
                text.push_str(&crate::prefix::truncation_marker(dropped));
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
            ));
        }
    };
    Ok(four)
}

/// The crash-recovery detection half of resume: a
/// `tool_called` with no later `tool_result` in the same run is a call
/// whose outcome is unknown.
pub fn dangling_tool_calls(ledger: &VerifiedLedger) -> Vec<(RunId, Seq)> {
    let mut pending: std::collections::BTreeMap<[u8; 16], (RunId, Seq)> =
        std::collections::BTreeMap::new();
    let mut dangling = Vec::new();
    for line in ledger.lines() {
        let VerifiedLine::Known { record, .. } = line else {
            continue;
        };
        let key = *record.run().as_bytes();
        match record.kind() {
            EventKind::ToolCalled => {
                if let Some(older) = pending.insert(key, (record.run(), record.seq())) {
                    dangling.push(older);
                }
            }
            EventKind::ToolResult => {
                pending.remove(&key);
            }
            _ => {}
        }
    }
    dangling.extend(pending.into_values());
    dangling.sort_by_key(|(_, seq)| seq.value());
    dangling
}

/// The repair half: the `tool_result` draft that closes a dangling call
/// with `E_TOOL_OUTCOME_UNKNOWN`. The resume path appends it before any
/// new turn — the account never shows a call without an outcome.
pub fn outcome_unknown_draft(call: &EventRecord, t: TimeMs) -> Result<EventDraft, AxError> {
    if call.kind() != EventKind::ToolCalled {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "draft unknown outcome",
            "record is not a tool_called",
        ));
    }
    let id = call
        .data()
        .as_map()
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let name = call
        .data()
        .as_map()
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let error = AxError::failure(
        AxCode::ToolOutcomeUnknown,
        "recover tool outcome",
        format!("{name} ({id})"),
    )
    .with_recovery(
        "the call may or may not have taken effect; verify the external state before retrying",
    );
    let mut map = serde_json::Map::new();
    map.insert("tool_use_id".to_owned(), serde_json::Value::String(id));
    map.insert("name".to_owned(), serde_json::Value::String(name));
    map.insert(
        "error".to_owned(),
        serde_json::to_value(&error).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "encode unknown outcome",
                err.to_string(),
            )
        })?,
    );
    Ok(EventDraft {
        run: call.run(),
        t,
        who: call.who().to_owned(),
        addr: None,
        kind: EventKind::ToolResult,
        data: kernel::Payload::new(map)?,
        ig: false,
    })
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
