// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::consts_external::EVENT_LOG_V;
use crate::error::{AxCode, AxError};
use crate::locator::B3Hash;

use super::identity::{RunId, Seq, TimeMs};
use super::kind::EventKind;

const PAYLOAD_DEPTH_MAX: usize = 126;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Payload(serde_json::Map<String, serde_json::Value>);

impl Payload {

    pub fn new(map: serde_json::Map<String, serde_json::Value>) -> Result<Self, AxError> {
        reject_floats_and_depth(&map)?;
        Ok(Payload(map))
    }

    pub fn empty() -> Self {
        Payload(serde_json::Map::new())
    }

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

    pub fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T, AxError> {
        T::deserialize(&self.0).map_err(|err| {
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

    pub fn into_map(self) -> serde_json::Map<String, serde_json::Value> {
        self.0
    }
}

impl<'de> Deserialize<'de> for Payload {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::deserialize(deserializer)?;
        Payload::new(map).map_err(serde::de::Error::custom)
    }
}

fn reject_floats_and_depth(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AxError> {

    let mut pending: Vec<(&serde_json::Value, usize)> =
        map.values().map(|value| (value, 1)).collect();
    while let Some((value, above)) = pending.pop() {
        match value {
            serde_json::Value::Number(n) if !n.is_i64() && !n.is_u64() => {
                return Err(
                    AxError::failure(AxCode::InvalidArgs, "build payload", n.to_string())
                        .with_recovery("ledger payloads never carry floats; scale to integers"),
                );
            }
            serde_json::Value::Array(items) => {
                let level = opened(above)?;
                pending.extend(items.iter().map(|item| (item, level)));
            }
            serde_json::Value::Object(inner) => {
                let level = opened(above)?;
                pending.extend(inner.values().map(|item| (item, level)));
            }
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }
    Ok(())
}

fn opened(above: usize) -> Result<usize, AxError> {
    above
        .checked_add(1)
        .filter(|level| *level <= PAYLOAD_DEPTH_MAX)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "build payload",
                format!("nesting deeper than {PAYLOAD_DEPTH_MAX} levels"),
            )
            .with_recovery(
                "store the body in CAS and carry its locator in the payload; the ledger reader cannot parse this depth back",
            )
        })
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDraft {
    pub run: RunId,
    pub t: TimeMs,
    pub who: String,
    pub addr: Option<Address>,
    pub kind: EventKind,
    pub data: Payload,

    pub ig: bool,
}

fn ig_is_false(ig: &bool) -> bool {
    !*ig
}

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

    pub fn parse_line(raw: &[u8]) -> Result<Self, AxError> {
        serde_json::from_slice(raw).map_err(|e| {
            AxError::failure(AxCode::InvalidArgs, "parse event line", e.to_string())
                .with_recovery("the line is not a canonical EventRecord")
        })
    }

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
