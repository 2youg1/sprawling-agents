// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one per-line check every Ledger reader applies: the envelope
//! (version direction, chain, seq), the kind split (typed or ignorable),
//! and the writer-canonical echo. `JsonlLedger::open`'s tail scan and
//! `runtime::replay` both walk a ledger through a `LineCheck`, so a line
//! one of them accepts the other cannot refuse (storage-SPEC 8-1).
//!
//! The check holds the chain state only - the previous line's hash and
//! the seq expected next - never a line, so a reader that feeds it one
//! line at a time holds one line at a time.

use std::borrow::Cow;

use kernel::consts_external::{LogVersion, readable_log_v};
use kernel::ledger::chain_hash;
use kernel::{AxCode, AxError, B3Hash, EventKind, EventRecord, GENESIS_PREV, Seq};
use serde::Deserialize;
use serde::de::IntoDeserializer;

/// Chain state between two lines: what the next line must continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCheck {
    prev: B3Hash,
    expected: Seq,
}

/// A line that passed: a typed record, or an explicitly ignorable line
/// from a newer vocabulary that still counts in the chain.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckedLine {
    Known(EventRecord),
    IgnoredUnknown(Seq),
}

/// Why a line did not pass, in the order the check asks.
#[derive(Debug, Clone, PartialEq)]
pub enum LineFault {
    /// The bytes do not carry a ledger envelope: torn or foreign bytes.
    NotALine(String),
    /// Written by a newer build (`readable_log_v` answers `Ahead`).
    VersionAhead(u64),
    /// Below the first version any build wrote.
    NotAVersion(u64),
    /// `prev` does not hash the previous line.
    ChainBreak,
    /// The seq is not the one the chain expects.
    SeqGap { found: Seq, expected: Seq },
    /// An unknown kind without `ig:true`: a newer writer's vocabulary.
    UnknownKind(String),
    /// A known kind whose record does not parse, or whose bytes are not
    /// what the writer would have written.
    NotCanonical(String),
    /// The seq space is exhausted after this line.
    SeqExhausted(AxError),
}

/// Enough of any line to judge version, chain and kind before a typed
/// parse. Unknown extra fields pass: this shape reads lines from the future.
#[derive(Deserialize)]
struct Envelope<'a> {
    v: u64,
    seq: Seq,
    prev: B3Hash,
    #[serde(borrow)]
    kind: Cow<'a, str>,
    #[serde(default)]
    ig: bool,
}

impl LineCheck {
    /// The state before a ledger's first line.
    pub fn at_genesis() -> Self {
        Self::after(GENESIS_PREV, Seq::FIRST)
    }

    /// The state after a line already verified elsewhere.
    pub(crate) fn after(prev: B3Hash, expected: Seq) -> Self {
        Self { prev, expected }
    }

    /// The seq the next line must carry.
    pub fn expected(&self) -> Seq {
        self.expected
    }

    pub(crate) fn prev(&self) -> B3Hash {
        self.prev
    }

    /// Whether `raw` carries a ledger envelope at all: a torn write never
    /// does, so a line that does is history, whatever else is wrong with it.
    pub(crate) fn carries_envelope(raw: &[u8]) -> bool {
        serde_json::from_slice::<Envelope>(raw).is_ok()
    }

    /// Judge one line (without its `\n`) and, when it passes, advance the
    /// chain past it. A fault leaves the state where it was.
    pub fn advance(&mut self, raw: &[u8]) -> Result<CheckedLine, LineFault> {
        let envelope = readable_envelope(raw)?;
        if envelope.prev != self.prev {
            return Err(LineFault::ChainBreak);
        }
        if envelope.seq != self.expected {
            return Err(LineFault::SeqGap {
                found: envelope.seq,
                expected: self.expected,
            });
        }
        let checked = classify(envelope, raw)?;
        self.expected = self.expected.next().map_err(LineFault::SeqExhausted)?;
        self.prev = chain_hash(raw);
        Ok(checked)
    }

    /// Everything `advance` asks of one line except where it sits in the
    /// chain, for a reader that links the chain from the other end
    /// (`jsonl::tail`): the line passes or fails here exactly as it
    /// would going forward.
    pub(super) fn judge(raw: &[u8]) -> Result<Judged, LineFault> {
        let envelope = readable_envelope(raw)?;
        let (seq, prev) = (envelope.seq, envelope.prev);
        let checked = classify(envelope, raw)?;
        Ok(Judged { seq, prev, checked })
    }
}

/// One line judged on its own: the chain position it claims, and what
/// it is.
pub(super) struct Judged {
    pub(super) seq: Seq,
    pub(super) prev: B3Hash,
    pub(super) checked: CheckedLine,
}

/// Judge one line (without its newline) as a record this build reads,
/// without the chain: the same answer `LineCheck::advance` gives about
/// the line's own bytes, for a reader that reached the line through an
/// index whose fold already walked the chain.
pub fn read_line(raw: &[u8]) -> Result<CheckedLine, LineFault> {
    readable_envelope(raw).and_then(|envelope| classify(envelope, raw))
}

fn readable_envelope(raw: &[u8]) -> Result<Envelope<'_>, LineFault> {
    let envelope: Envelope = serde_json::from_slice(raw)
        .map_err(|e| LineFault::NotALine(format!("not a ledger line: {e}")))?;
    match readable_log_v(envelope.v) {
        LogVersion::Current | LogVersion::Older => Ok(envelope),
        LogVersion::Ahead => Err(LineFault::VersionAhead(envelope.v)),
        LogVersion::NotAVersion => Err(LineFault::NotAVersion(envelope.v)),
    }
}

/// The one rule for what a line of a readable envelope is: a record of a
/// kind this build knows, an ignorable line of a newer kind, or a fault.
fn classify(envelope: Envelope<'_>, raw: &[u8]) -> Result<CheckedLine, LineFault> {
    let known = EventKind::deserialize(
        IntoDeserializer::<serde::de::value::Error>::into_deserializer(envelope.kind.as_ref()),
    );
    match known {
        Ok(_) => canonical_record(raw).map(CheckedLine::Known),
        Err(_) if envelope.ig => Ok(CheckedLine::IgnoredUnknown(envelope.seq)),
        Err(_) => Err(LineFault::UnknownKind(envelope.kind.into_owned())),
    }
}

fn canonical_record(raw: &[u8]) -> Result<EventRecord, LineFault> {
    let record = EventRecord::parse_line(raw)
        .map_err(|e| LineFault::NotCanonical(format!("typed parse failed: {e}")))?;
    let echo = record
        .canonical_line()
        .map_err(|e| LineFault::NotCanonical(format!("no canonical form: {e}")))?;
    if echo != raw {
        return Err(LineFault::NotCanonical(
            "bytes are not writer-canonical".to_string(),
        ));
    }
    Ok(record)
}

impl LineFault {
    /// The refusal a reader that walks a whole ledger gives, naming the
    /// 1-based line: a newer writer speaks direction
    /// (`E_LOG_VERSION_UNSUPPORTED`), everything else is storage
    /// integrity (`E_CAS_CORRUPT`).
    pub fn into_ax(self, line_no: u64) -> AxError {
        let newer = |detail: String| {
            AxError::failure(AxCode::LogVersionUnsupported, "verify ledger", detail).with_recovery(
                "written by a newer sprawling; replay it with the version that wrote it",
            )
        };
        let corrupt = |violation: String| {
            AxError::failure(
                AxCode::CasCorrupt,
                "verify ledger",
                format!("line {line_no}"),
            )
            .with_recovery(violation)
        };
        match self {
            Self::VersionAhead(v) => newer(format!("line {line_no} carries v{v}")),
            Self::UnknownKind(kind) => AxError::failure(
                AxCode::LogVersionUnsupported,
                "verify ledger",
                format!("line {line_no} kind `{kind}`"),
            )
            .with_recovery(
                "unknown kind without ig:true means a newer writer; use the sprawling that wrote it",
            ),
            Self::NotAVersion(v) => corrupt(format!("impossible v{v}")),
            Self::NotALine(violation) | Self::NotCanonical(violation) => corrupt(violation),
            Self::ChainBreak => corrupt("prev does not hash the previous line".to_string()),
            Self::SeqGap { found, expected } => corrupt(format!(
                "seq {} where {} was expected",
                found.value(),
                expected.value()
            )),
            Self::SeqExhausted(source) => source,
        }
    }
}
