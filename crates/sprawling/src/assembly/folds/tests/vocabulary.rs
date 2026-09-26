// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A history that verifies is a history a city starts from.

use super::super::*;
use crate::assembly::*;

/// A newer writer's line that marks itself ignorable passes the one
/// per-line check; the folds read what that check read, so the city
/// opens past it instead of refusing a line it was told it may skip.
#[test]
fn a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let segment = memory::ledger_segments_at(&report.ledger_dir)
        .unwrap()
        .pop()
        .unwrap();
    let mut bytes = std::fs::read(&segment).unwrap();
    let lines: Vec<&[u8]> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    let future = format!(
        "{{\"v\":1,\"run\":\"00000000-0000-0000-0000-000000000000\",\"seq\":{},\
         \"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\
         \"data\":{{}},\"ig\":true}}\n",
        lines.len(),
        kernel::ledger::chain_hash(lines.last().unwrap())
    );
    bytes.extend_from_slice(future.as_bytes());
    std::fs::write(&segment, &bytes).unwrap();

    let views = rebuild_views(&report.ledger_dir).map(|_| ());
    let standing = Standing::fold(&report.ledger_dir).map(|_| ());
    assert_eq!((views, standing), (Ok(()), Ok(())));
}

/// A pursuit line this build cannot read stops the fold. Skipping it
/// would leave standing whatever goal the line changed, so a building a
/// person cleared would go on pursuing after a restart.
#[test]
fn a_pursuit_line_the_build_cannot_read_stops_the_fold() {
    let line = unreadable(
        kernel::EventKind::PursuitChanged,
        serde_json::json!({ "step": "abandon", "goal": "read the meter" }),
    );
    assert!(CollaborationFold::default().absorb(&line).is_err());
}

/// A goal line this build cannot read is a mismatch of vocabulary, the
/// same refusal every other record read gives, so the recovery a person
/// is handed is to replay with the build that wrote it.
#[test]
fn a_goal_line_the_build_cannot_read_is_a_wire_mismatch() {
    let line = unreadable(
        kernel::EventKind::GoalRegistered,
        serde_json::json!({ "id": "a", "owner": "lab/a" }),
    );
    let refused = CollaborationFold::default()
        .absorb(&line)
        .map_err(|err| err.code().clone());
    assert_eq!(refused, Err(kernel::AxCode::WireMismatch));
}

fn unreadable(kind: kernel::EventKind, data: serde_json::Value) -> EventRecord {
    let draft = kernel::EventDraft {
        run: RunId::CITY,
        t: kernel::TimeMs::new(0),
        who: "person".into(),
        addr: Some(Address::parse("lab").unwrap()),
        kind,
        data: kernel::Payload::new(data.as_object().unwrap().clone()).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, kernel::Seq::FIRST, kernel::ledger::GENESIS_PREV)
}
