// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;

use kernel::consts_external::{LogVersion, readable_log_v};
use kernel::ledger::chain_hash;
use kernel::{AxCode, AxError, B3Hash, EventKind, EventRecord, GENESIS_PREV, Seq};
use serde::Deserialize;
use serde::de::IntoDeserializer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCheck {
    prev: B3Hash,
    expected: Seq,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedLine {
    Known(EventRecord),
    IgnoredUnknown(Seq),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineFault {
    NotALine(String),

    VersionAhead(u64),

    NotAVersion(u64),

    ChainBreak,

    SeqGap { found: Seq, expected: Seq },

    UnknownKind(String),

    NotCanonical(String),

    SeqExhausted(AxError),
}

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
    pub fn at_genesis() -> Self {
        Self::after(GENESIS_PREV, Seq::FIRST)
    }

    pub(crate) fn after(prev: B3Hash, expected: Seq) -> Self {
        Self { prev, expected }
    }

    pub fn expected(&self) -> Seq {
        self.expected
    }

    pub(crate) fn prev(&self) -> B3Hash {
        self.prev
    }

    pub(crate) fn carries_envelope(raw: &[u8]) -> bool {
        raw.rsplit(|byte| *byte == 0)
            .next()
            .is_some_and(|after| serde_json::from_slice::<Envelope>(after).is_ok())
    }

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

    pub(super) fn judge(raw: &[u8]) -> Result<Judged, LineFault> {
        let envelope = readable_envelope(raw)?;
        let (seq, prev) = (envelope.seq, envelope.prev);
        let checked = classify(envelope, raw)?;
        Ok(Judged { seq, prev, checked })
    }
}

pub(super) struct Judged {
    pub(super) seq: Seq,
    pub(super) prev: B3Hash,
    pub(super) checked: CheckedLine,
}

pub fn read_line(raw: &[u8]) -> Result<CheckedLine, LineFault> {
    readable_envelope(raw).and_then(|envelope| classify(envelope, raw))
}

pub(crate) fn claimed_seq(raw: &[u8]) -> Option<Seq> {
    readable_envelope(raw).ok().map(|envelope| envelope.seq)
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
