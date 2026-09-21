// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Event payloads, drafts, records, and refs.

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::consts_external::EVENT_LOG_V;
use crate::error::{AxCode, AxError};
use crate::locator::B3Hash;

use super::identity::{RunId, Seq, TimeMs};
use super::kind::EventKind;

/// Ledger payload: a JSON object with every float refused, at construction
/// and again on read (determinism rule 6). Keys serialize sorted
/// (serde_json's default BTreeMap), which is part of the canonical bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Payload(serde_json::Map<String, serde_json::Value>);

impl Payload {
    /// Sole constructor.
    pub fn new(map: serde_json::Map<String, serde_json::Value>) -> Result<Self, AxError> {
        for value in map.values() {
            reject_floats(value)?;
        }
        Ok(Payload(map))
    }

    pub fn empty() -> Self {
        Payload(serde_json::Map::new())
    }

    /// The way in: one of the record structs in [`crate::event::record`],
    /// encoded as the object a ledger line carries. Every writer goes
    /// through here, so the keys of a kind are spelled in exactly one
    /// place: the struct.
    ///
    /// # Errors
    /// Refuses a record that does not encode to a JSON object, and any
    /// float inside one, for the reasons [`Payload::new`] gives.
    pub fn of(record: &impl Serialize) -> Result<Self, AxError> {
        let value = serde_json::to_value(record).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "encode a ledger payload",
                err.to_string(),
            )
            .with_recovery("give the record string keys and integer numbers only")
        })?;
        let serde_json::Value::Object(map) = value else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "encode a ledger payload",
                "the record did not encode to a JSON object",
            )
            .with_recovery("a ledger payload is an object; record this fact as a struct"));
        };
        Payload::new(map)
    }

    /// The way out: the record struct this payload was written from.
    ///
    /// A field a record written by an older build may omit carries
    /// `#[serde(default)]` on the struct, so the tolerance is declared
    /// once per key rather than at each reader. Unknown keys pass, so a
    /// newer writer never breaks an older reader.
    ///
    /// # Errors
    /// A payload this build cannot read as `T`.
    pub fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T, AxError> {
        serde_json::from_value(serde_json::Value::Object(self.0.clone())).map_err(|err| {
            AxError::failure(
                AxCode::WireMismatch,
                "read a ledger payload",
                err.to_string(),
            )
            .with_recovery("replay with the build that wrote this record")
        })
    }

    pub fn as_map(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Payload {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::deserialize(deserializer)?;
        Payload::new(map).map_err(serde::de::Error::custom)
    }
}

fn reject_floats(value: &serde_json::Value) -> Result<(), AxError> {
    match value {
        serde_json::Value::Number(n) if !n.is_i64() && !n.is_u64() => {
            Err(
                AxError::failure(AxCode::InvalidArgs, "build payload", n.to_string())
                    .with_recovery("ledger payloads never carry floats; scale to integers"),
            )
        }
        serde_json::Value::Array(items) => items.iter().try_for_each(reject_floats),
        serde_json::Value::Object(map) => map.values().try_for_each(reject_floats),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => Ok(()),
    }
}

/// What a recording party supplies; the Ledger implementation owns the
/// rest (seq, prev, v).
#[derive(Debug, Clone, PartialEq)]
pub struct EventDraft {
    pub run: RunId,
    pub t: TimeMs,
    pub who: String,
    pub addr: Option<Address>,
    pub kind: EventKind,
    pub data: Payload,
    /// "Ignorable": a future reader may skip this line without changing
    /// any rebuild result. Writers set it only when that holds.
    pub ig: bool,
}

fn ig_is_false(ig: &bool) -> bool {
    !*ig
}

/// One serialized history line. Fields private: the only paths to a
/// record are [`EventRecord::from_draft`] (append side) and
/// [`EventRecord::parse_line`] (read side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EventRecord {
    v: u32,
    run: RunId,
    seq: Seq,
    prev: B3Hash,
    t: TimeMs,
    who: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    addr: Option<Address>,
    kind: EventKind,
    data: Payload,
    #[serde(skip_serializing_if = "ig_is_false", default)]
    ig: bool,
}

impl EventRecord {
    /// Append-side assembly: the caller (a Ledger implementation) owns seq
    /// and prev; `v` is pinned to [`EVENT_LOG_V`].
    pub fn from_draft(draft: EventDraft, seq: Seq, prev: B3Hash) -> Self {
        EventRecord {
            v: EVENT_LOG_V,
            run: draft.run,
            seq,
            prev,
            t: draft.t,
            who: draft.who,
            addr: draft.addr,
            kind: draft.kind,
            data: draft.data,
            ig: draft.ig,
        }
    }

    /// The canonical bytes (no line terminator). The one byte producer
    /// library-wide; adapters append these bytes verbatim and hash exactly
    /// them for the next line's prev.
    pub fn canonical_line(&self) -> Result<Vec<u8>, AxError> {
        serde_json::to_vec(self).map_err(|e| {
            AxError::failure(
                AxCode::InvalidArgs,
                "serialize event",
                format!("{:?} at seq {}", self.kind, self.seq.value()),
            )
            .with_recovery(format!(
                "JSON refused this payload ({e}); give the event's data only string keys and finite numbers"
            ))
        })
    }

    /// Read-side entrance: full field revalidation, fail-closed. Unknown
    /// kinds fail here; the `ig:true` skip belongs to replay, which probes
    /// the envelope before committing to a typed parse.
    pub fn parse_line(raw: &[u8]) -> Result<Self, AxError> {
        serde_json::from_slice(raw).map_err(|e| {
            AxError::failure(AxCode::InvalidArgs, "parse event line", e.to_string())
                .with_recovery("the line is not a canonical v1 EventRecord")
        })
    }

    /// Minting a reference requires holding a whole record (15.3-1): the
    /// append path holds what it just assembled, replay holds what it just
    /// verified. There is no third path.
    pub fn to_ref(&self) -> EventRef {
        EventRef {
            seq: self.seq,
            kind: self.kind,
        }
    }

    pub fn v(&self) -> u32 {
        self.v
    }

    pub fn run(&self) -> RunId {
        self.run
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn prev(&self) -> B3Hash {
        self.prev
    }

    pub fn t(&self) -> TimeMs {
        self.t
    }

    pub fn who(&self) -> &str {
        &self.who
    }

    pub fn addr(&self) -> Option<&Address> {
        self.addr.as_ref()
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn data(&self) -> &Payload {
        &self.data
    }

    pub fn ig(&self) -> bool {
        self.ig
    }
}

/// Unforgeable pointer at a ledger line: private fields, no public
/// constructor, no serde (deserialization would be a third mint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventRef {
    seq: Seq,
    kind: EventKind,
}

impl EventRef {
    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn kind(&self) -> EventKind {
        self.kind
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
