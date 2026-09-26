// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

/// A kind that left the vocabulary is refused where lines are read, and
/// the refusal carries a stable code and a way forward rather than a
/// serde sentence. `takeover_started` and `rollback_applied` left with
/// the commands behind them (kernel-SPEC.md section 12.2); no line
/// carrying them was ever written, so nothing already on disk changes,
/// and a hand-written one is refused rather than read.
#[test]
fn a_kind_that_left_the_vocabulary_is_refused_with_a_stable_code() {
    for retired in ["takeover_started", "rollback_applied"] {
        let raw = format!(
            r#"{{"v":1,"run":"00000000-0000-0000-0000-000000000000","seq":0,"prev":"0000000000000000000000000000000000000000000000000000000000000000","t":0,"who":"x","kind":"{retired}","data":{{}}}}"#
        );
        let err = EventRecord::parse_line(raw.as_bytes())
            .expect_err("a retired kind is not a line of this history");
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(
            !err.recovery().is_empty(),
            "the refusal owes a way forward: {err}"
        );
    }
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

/// A payload whose root object and nested containers count `depth`.
fn nested(depth: usize) -> Map<String, Value> {
    let inner = (1..depth).fold(json!(0), |value, _| json!({ "k": value }));
    let mut root = Map::new();
    root.insert("k".into(), inner);
    root
}

/// The read side is serde_json with its 128-container limit, and the
/// envelope spends one of them: a payload at the limit reads back, and
/// one past it is refused where it is written rather than where it is
/// replayed, with the way out that keeps the body.
#[test]
fn a_payload_nested_past_what_the_reader_can_parse_is_refused_on_write() {
    let at_limit = EventDraft {
        data: Payload::new(nested(PAYLOAD_DEPTH_MAX)).unwrap(),
        ..draft(EventKind::CityInitialized)
    };
    let record = EventRecord::from_draft(at_limit, Seq::FIRST, B3Hash::from_bytes([0; 32]));
    let line = record.canonical_line().unwrap();
    assert_eq!(EventRecord::parse_line(&line).unwrap(), record);

    for depth in [PAYLOAD_DEPTH_MAX + 1, 130] {
        let err = Payload::new(nested(depth)).expect_err("the reader could not parse this back");
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("CAS"), "{err}");
    }
}

/// The float check descends one call per level, and the depth check does
/// not, so the depth is judged first: a payload too deep for the reader
/// is refused for its depth even when a float waits at the bottom, and
/// the recursive walk only ever runs over the levels the reader accepts.
#[test]
fn a_payload_too_deep_is_refused_for_its_depth_before_its_floats_are_walked() {
    let inner = (1..PAYLOAD_DEPTH_MAX + 1).fold(json!(0.5), |value, _| json!([value]));
    let mut deep = Map::new();
    deep.insert("k".into(), inner);
    let err = Payload::new(deep).expect_err("too deep and carrying a float");
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("CAS"), "{err}");
}
