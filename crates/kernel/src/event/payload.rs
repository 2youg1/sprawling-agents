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
        _ => Ok(()),
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
        serde_json::to_vec(self)
            .map_err(|e| AxError::failure(AxCode::InvalidArgs, "serialize event", e.to_string()))
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
mod tests {
    use super::*;
    use crate::error::AxCode;
    use crate::locator::B3Hash;
    use serde_json::{Map, Value, json};
    fn draft(kind: EventKind) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(0),
            who: "city".to_string(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        }
    }

    #[test]
    fn payload_rejects_floats_anywhere() {
        let mut ok = Map::new();
        ok.insert("n".into(), json!(42));
        ok.insert("s".into(), json!("text"));
        ok.insert("list".into(), json!([1, 2, {"deep": true}]));
        assert!(Payload::new(ok).is_ok());

        for bad in [
            json!({"x": 1.5}),
            json!({"x": [1, 2.0]}),
            json!({"x": {"y": {"z": 0.1}}}),
        ] {
            let Value::Object(map) = bad else {
                panic!("test data must be objects")
            };
            let err = Payload::new(map).unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs);
        }
    }

    #[test]
    fn payload_deserialize_revalidates() {
        assert!(serde_json::from_str::<Payload>(r#"{"a":1}"#).is_ok());
        assert!(serde_json::from_str::<Payload>(r#"{"a":1.5}"#).is_err());
    }

    #[test]
    fn canonical_line_is_stable_and_omits_empty_optionals() {
        let record = EventRecord::from_draft(
            draft(EventKind::CityInitialized),
            Seq::FIRST,
            B3Hash::from_bytes([0; 32]),
        );
        let line = record.canonical_line().unwrap();
        let text = String::from_utf8(line.clone()).unwrap();
        assert!(text.starts_with("{\"v\":1,\"run\":\""), "{text}");
        assert!(
            !text.contains("\"addr\""),
            "None addr must be omitted: {text}"
        );
        assert!(!text.contains("\"ig\""), "false ig must be omitted: {text}");
        assert!(!text.ends_with('\n'), "line terminator is the adapter's");
        insta::assert_snapshot!("genesis_line", text);
    }

    #[test]
    fn canonical_line_keeps_declaration_order_with_addr_and_ig() {
        let mut d = draft(EventKind::BuildingCreated);
        d.addr = Some(crate::address::Address::parse("lab").unwrap());
        d.ig = true;
        let record =
            EventRecord::from_draft(d, Seq::FIRST.next().unwrap(), B3Hash::from_bytes([17; 32]));
        let text = String::from_utf8(record.canonical_line().unwrap()).unwrap();
        let order = [
            "\"v\"", "\"run\"", "\"seq\"", "\"prev\"", "\"t\"", "\"who\"", "\"addr\"", "\"kind\"",
            "\"data\"", "\"ig\"",
        ];
        let mut last = 0;
        for key in order {
            let at = text
                .find(key)
                .unwrap_or_else(|| panic!("{key} missing in {text}"));
            assert!(at >= last, "{key} out of order in {text}");
            last = at;
        }
        insta::assert_snapshot!("building_created_line", text);
    }

    #[test]
    fn parse_line_roundtrips_canonical_bytes() {
        let record = EventRecord::from_draft(
            draft(EventKind::RunStarted),
            Seq::FIRST,
            B3Hash::from_bytes([0; 32]),
        );
        let line = record.canonical_line().unwrap();
        let back = EventRecord::parse_line(&line).unwrap();
        assert_eq!(back, record);
        assert_eq!(back.canonical_line().unwrap(), line);
    }

    #[test]
    fn parse_line_fails_closed() {
        assert!(EventRecord::parse_line(b"not json").is_err());
        assert!(
            EventRecord::parse_line(
                br#"{"v":1,"run":"00000000-0000-0000-0000-000000000000","seq":0,"prev":"00","t":0,"who":"x","kind":"city_initialized","data":{}}"#
            )
            .is_err(),
            "short prev hex must fail"
        );
        assert!(
            EventRecord::parse_line(
                br#"{"v":1,"run":"00000000-0000-0000-0000-000000000000","seq":0,"prev":"0000000000000000000000000000000000000000000000000000000000000000","t":0,"who":"x","kind":"no_such_kind","data":{}}"#
            )
            .is_err(),
            "unknown kind must fail here; ig-skip lives in replay"
        );
        assert!(
            EventRecord::parse_line(
                br#"{"v":1,"run":"00000000-0000-0000-0000-000000000000","seq":0,"prev":"0000000000000000000000000000000000000000000000000000000000000000","t":0,"who":"x","kind":"city_initialized","data":{"f":1.25}}"#
            )
            .is_err(),
            "float payload must fail on read too"
        );
    }

    #[test]
    fn event_ref_reports_the_record_it_was_minted_from() {
        let record = EventRecord::from_draft(
            draft(EventKind::GateChecked),
            Seq::FIRST,
            B3Hash::from_bytes([0; 32]),
        );
        let echo = record.to_ref();
        assert_eq!(echo.seq(), record.seq());
        assert_eq!(echo.kind(), EventKind::GateChecked);
    }
}
