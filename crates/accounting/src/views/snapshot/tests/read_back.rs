// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A history with listed model facts reads back from the views snapshot
//! (kernel D58, `crates/accounting/spec/Views/Snapshot.lean`).

use kernel::{Address, B3Hash, EventKind, RunId};

use crate::views::Views;
use crate::views::tests::{Place, view_record};

/// A 0.0.10 history, then the 0.0.11 lines that carry model facts and a
/// harness agent, folded and read back from the views snapshot (F-04):
/// an absent `thinking` or `canonical` once shifted every later field of
/// the postcard encoding, so every start after an attach refused the
/// snapshot and folded from genesis.
#[test]
fn listed_model_facts_and_a_harness_agent_read_back_from_the_views_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let facts = concat!(
        r#"[{"canonical":"vendor/m-new","context_tokens":200000,"id":"m-new","#,
        r#""input_modalities":["text"],"input_price":null,"max_output_tokens":null,"#,
        r#""output_price":null,"thinking":{"default":"medium","levels":["low","medium","high"]}},"#,
        r#"{"context_tokens":null,"id":"m-switch","input_modalities":[],"input_price":null,"#,
        r#""max_output_tokens":null,"output_price":null,"thinking":{"on":true}}]"#,
    );
    let blob = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas())
        .unwrap()
        .put(facts.as_bytes())
        .unwrap();
    let old = RunId::from_bytes([7; 16]);
    let harness = RunId::from_bytes([8; 16]);
    let lines = [
        (
            old,
            EventKind::EndpointAttached,
            serde_json::json!({
                "auth": "secret:providers/old", "base_url": "https://api.example.test/v1",
                "dialect": "open_ai", "models": ["m-old"], "name": "old"
            }),
        ),
        (
            old,
            EventKind::RunStarted,
            serde_json::json!({"task": "fixture", "effort": "high"}),
        ),
        (
            RunId::CITY,
            EventKind::EndpointAttached,
            serde_json::json!({
                "base_url": "https://api.example.test/v2", "dialect": "open_ai",
                "facts_blob": blob.to_string(), "models": ["m-new", "m-switch"], "name": "new"
            }),
        ),
        (
            harness,
            EventKind::RunStarted,
            serde_json::json!({"task": "fixture", "agent": {
                "id": "fixture-agent", "launch_digest": B3Hash::digest(b"spec").to_string()
            }}),
        ),
    ];
    let mut views = Views::new(dir.path());
    for (seq, (run, kind, data)) in (1..).zip(lines) {
        let serde_json::Value::Object(data) = data else {
            panic!("each line is an object");
        };
        views
            .apply(&view_record(Place { seq, run }, kind, &room, data))
            .unwrap();
    }

    let decoded = Views::decode(dir.path(), &views.encode().unwrap()).unwrap();

    let learned: Vec<kernel::event::record::ModelFacts> = serde_json::from_str(facts).unwrap();
    let listed = |book: &gateway::EndpointBook| {
        book.endpoints()
            .map(|endpoint| (endpoint.name.clone(), endpoint.models.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        (
            listed(&decoded.book),
            serde_json::to_value(&decoded.book).unwrap()
        ),
        (
            vec![
                ("new".to_owned(), learned),
                (
                    "old".to_owned(),
                    vec![kernel::event::record::ModelFacts {
                        id: "m-old".to_owned(),
                        ..kernel::event::record::ModelFacts::default()
                    }]
                ),
            ],
            serde_json::to_value(&views.book).unwrap()
        )
    );
}
