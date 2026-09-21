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
