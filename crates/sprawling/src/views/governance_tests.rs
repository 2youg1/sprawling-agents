// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two readings this wave added: who answers for a city and what was
//! answered for the person, and one file's patch text.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::views::Views;
use kernel::{Address, EventKind, RunId};

use super::tests::view_record;

/// Who answers for this city, and what was answered on the person's
/// behalf, are one question with one answer (card-5.4).
#[test]
fn the_governance_view_reports_who_answers_and_what_was_answered() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([7u8; 16]);

    let mut delegated = serde_json::Map::new();
    delegated.insert(
        "scope".to_owned(),
        serde_json::Value::String("city".to_owned()),
    );
    delegated.insert(
        "autonomy".to_owned(),
        serde_json::Value::String("delegate:hall/clerk".to_owned()),
    );
    views
        .apply(&view_record(
            1,
            run,
            EventKind::AutonomyChanged,
            &room,
            delegated,
        ))
        .unwrap();

    let mut answered = serde_json::Map::new();
    answered.insert(
        "id".to_owned(),
        serde_json::Value::String("item-1".to_owned()),
    );
    answered.insert(
        "verdict".to_owned(),
        serde_json::Value::String("allow".to_owned()),
    );
    answered.insert(
        "cluster".to_owned(),
        serde_json::json!({ "class": "agent_question", "detail": "lab/room1" }),
    );
    views
        .apply(&view_record(
            2,
            run,
            EventKind::ApprovalResolved,
            &room,
            answered,
        ))
        .unwrap();

    let channels::Answer::Governance(governance) = views.answer(&channels::Query::Governance)
    else {
        panic!("Governance answers with a governance reading");
    };
    assert!(
        matches!(governance.autonomy, kernel::Autonomy::Delegate(ref who) if who.as_str() == "hall/clerk"),
        "the delegate the ledger names is who answers"
    );
    assert_eq!(governance.decided.len(), 1, "one thing was decided");
    assert_eq!(governance.decided[0].item, "item-1");
    assert_eq!(governance.decided[0].verdict, kernel::PolicyVerdict::Allow);
    assert_eq!(governance.decided[0].cluster.detail, "lab/room1");
}

/// A commit this city never wrote is unavailable rather than empty: "it
/// changed nothing" and "I cannot read it" are different answers, and a
/// reader's next move differs (card-2.6).
#[test]
fn a_hunk_of_a_commit_this_city_never_wrote_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let answer = views.answer(&channels::Query::Hunks {
        oid_a: kernel::GitOid::from_bytes([1u8; 20]),
        oid_b: kernel::GitOid::from_bytes([2u8; 20]),
        path: "lab/lex.rs".to_owned(),
    });
    let channels::Answer::Unavailable { query } = answer else {
        panic!("a city with no repository cannot answer with a patch");
    };
    assert!(query.starts_with("Hunks("), "the answer names the question");
}
