// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `VIEWS_FOLD_RULES` moves whenever the encoding of `Views` does
//! (`crates/sprawling/Spec.lean` §8-91).

use kernel::{Address, EventKind, Payload, RunId};

use super::*;
use crate::views::tests::{Place, view_record};

/// A view folded from records that fill the inbox, the discard bin, the
/// registry, the commits and the governed rules, so a field added,
/// removed or reordered among them changes the bytes, and so does the
/// encoding of either digest a snapshot holds (`crates/accounting/spec/Views/Snapshot.lean` §8-24).
fn fixture(city_root: &Path) -> Views {
    let mut views = Views::new(city_root);
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let tracked = format!("file:lab/room1/notes.md@{}", "ab".repeat(20));
    let signal = collab::Signal::new(
        kernel::event::record::SignalId::parse("sig-1").unwrap(),
        kernel::event::record::SignalKind::Thread,
        "lab/room2".to_owned(),
        room.clone(),
        kernel::Version::new(1),
        kernel::Payload::new(serde_json::Map::new()).unwrap(),
        kernel::TimeMs::new(1_000),
    )
    .unwrap();
    let records = [
        (
            EventKind::SignalEnqueued,
            serde_json::Value::Object(signal.enqueued_payload().unwrap().as_map().clone()),
        ),
        (
            EventKind::FileDiscarded,
            serde_json::json!({"paths": ["file:lab/room1/notes.md"], "restoration": {"tracked": tracked}}),
        ),
        (
            EventKind::AssetArchived,
            serde_json::json!({"kind": "decision", "subject": "chose git over a second index"}),
        ),
        (
            EventKind::CheckpointCommitted,
            serde_json::json!({"oid": "cd".repeat(20)}),
        ),
        (EventKind::RulesChanged, rules_changed()),
    ];
    for (seq, (run, kind, data)) in (1..).zip(
        records
            .into_iter()
            .map(|(kind, data)| (run, kind, data))
            .chain(provider_registrations()),
    ) {
        let serde_json::Value::Object(data) = data else {
            panic!("each fixture payload is an object");
        };
        views
            .apply(&view_record(Place { seq, run }, kind, &room, data))
            .unwrap();
    }
    views
}

/// Registrations the endpoint book folds, one of them listing a model,
/// so the encoding of `ModelFacts` in the book is part of the digest
/// (kernel D58).
pub(crate) fn provider_registrations() -> [(RunId, EventKind, serde_json::Value); 13] {
    let old = RunId::from_bytes([7; 16]);
    let fresh = RunId::from_bytes([8; 16]);
    [
        (
            old,
            EventKind::EndpointAttached,
            serde_json::json!({
                "name": "listed", "base_url": "https://api.example.test/v1",
                "dialect": kernel::DialectKind::OpenAi, "models": ["m-listed"]
            }),
        ),
        (
            old,
            EventKind::EndpointAttached,
            serde_json::json!({
                "name": "legacy", "base_url": "https://api.example.test/v1",
                "dialect": kernel::DialectKind::OpenAi, "models": [], "auth": "secret:providers/legacy"
            }),
        ),
        (
            old,
            EventKind::EndpointAttached,
            serde_json::json!({
                "name": "explicit", "base_url": "https://api.example.test/v1",
                "dialect": kernel::DialectKind::OpenAi, "models": [], "tuning": {"accounts": [
                    {"id": "first", "reference": "secret:providers/first"},
                    {"id": "anonymous"}
                ]}
            }),
        ),
        (
            old,
            EventKind::RunStarted,
            serde_json::json!({"task": "fixture"}),
        ),
        (
            old,
            EventKind::ModelCalled,
            serde_json::json!({"provider_account": {"provider": "explicit", "account": "first"}, "model": "fixture", "segments": []}),
        ),
        (
            old,
            EventKind::ModelReturned,
            serde_json::json!({"message": {"content": []}, "calls": 0}),
        ),
        (
            old,
            EventKind::ModelCalled,
            serde_json::json!({"provider_account": {"provider": "explicit", "account": "anonymous"}, "model": "fixture", "segments": []}),
        ),
        (
            RunId::CITY,
            EventKind::SessionOpened,
            serde_json::json!({"carried": false}),
        ),
        (
            old,
            EventKind::ModelReturned,
            serde_json::json!({"message": {"content": []}, "calls": 0}),
        ),
        (
            fresh,
            EventKind::RunStarted,
            serde_json::json!({"task": "fixture"}),
        ),
        (
            fresh,
            EventKind::ModelCalled,
            serde_json::json!({"provider_account": {"provider": "explicit", "account": "first"}, "model": "fixture", "segments": []}),
        ),
        (
            fresh,
            EventKind::ModelReturned,
            serde_json::json!({"message": {"content": []}, "calls": 0}),
        ),
        (
            fresh,
            EventKind::ModelCalled,
            serde_json::json!({"provider_account": {"provider": "explicit", "account": "anonymous"}, "model": "fixture", "segments": []}),
        ),
    ]
}

#[test]
fn legacy_and_explicit_accounts_decode_from_the_actual_views_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let views = fixture(dir.path());
    let encoded = views.encode().unwrap();
    let decoded = Views::decode(dir.path(), &encoded);
    assert!(
        decoded.is_ok(),
        "the production snapshot must decode: {:?}",
        decoded.as_ref().err()
    );
    assert_eq!(
        serde_json::to_value(&decoded.unwrap().book).unwrap(),
        serde_json::to_value(&views.book).unwrap()
    );
}

/// The city's configuration booked as the digest of five bytes: the
/// governed rules hold a `B3Hash` (`crates/accounting/spec/Views/Snapshot.lean` §8-24).
pub(crate) fn rules_changed() -> serde_json::Value {
    let changed = kernel::event::record::RulesChanged {
        scope: kernel::event::Scope::City,
        which: kernel::event::record::GoverningDocument::Config,
        before: None,
        after: B3Hash::digest(b"rules"),
        bytes: 5,
    };
    serde_json::Value::Object(kernel::Payload::of(&changed).unwrap().as_map().clone())
}

#[test]
fn the_fold_rules_name_carries_the_digest_of_the_views_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let digest = B3Hash::digest(&fixture(dir.path()).encode().unwrap()).to_string();

    let expected = format!("views-fold-{}", digest.get(..16).unwrap());

    assert_eq!(
        VIEWS_FOLD_RULES, expected,
        "the encoding of Views changed: set VIEWS_FOLD_RULES to the expected value, \
         so every snapshot cut under the old encoding is refused"
    );
}

/// Views hold run ids - commit facts, predecessors, skill pins - and a
/// snapshot or a twin reads them back from the same binary encoding they
/// were written in. A city with one commit in it could not be served.
#[test]
fn a_run_id_reads_back_from_the_snapshot_encoding() {
    let run = RunId::from_bytes([7u8; 16]);
    let bytes = postcard::to_allocvec(&run).unwrap();
    assert_eq!(postcard::from_bytes::<RunId>(&bytes).unwrap(), run);
}

/// A snapshot holds a digest as its bytes and reads the same digest
/// back, while every format a reader sees still spells it in hex
/// (`crates/kernel/Spec.lean` §8-84): the hex decode was most of decoding the views.
#[test]
fn a_digest_is_its_bytes_in_the_snapshot_and_its_hex_in_json() {
    let oid = kernel::GitOid::from_bytes([0xcd; 20]);
    let hash = B3Hash::from_bytes([0x5a; 32]);
    let encoded = (
        postcard::to_allocvec(&oid).unwrap(),
        postcard::to_allocvec(&hash).unwrap(),
    );

    let read_back = (
        postcard::from_bytes::<kernel::GitOid>(&encoded.0).unwrap(),
        postcard::from_bytes::<B3Hash>(&encoded.1).unwrap(),
    );
    let spelled = (
        serde_json::to_string(&oid).unwrap(),
        serde_json::to_string(&hash).unwrap(),
    );

    assert_eq!(
        (encoded, read_back, spelled),
        (
            (vec![0xcd; 20], vec![0x5a; 32]),
            (oid, hash),
            (
                format!("\"{}\"", "cd".repeat(20)),
                format!("\"{}\"", "5a".repeat(32))
            ),
        )
    );
}

proptest::proptest! {
    #[test]
    fn closed_session_traces_preserve_current_bindings_across_replay_and_snapshot(
        late in proptest::collection::vec(0u8..12, 0..24),
        cut in 0usize..64,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        let neighbour = Address::parse("lab/room2").unwrap();
        let old = RunId::from_bytes([2; 16]);
        let spare = RunId::from_bytes([3; 16]);
        let other = RunId::from_bytes([4; 16]);
        let current = RunId::from_bytes([5; 16]);
        let called = |account: &str| Payload::of(&kernel::event::record::ModelCalled {
            model: "fixture".to_owned(), segments: Vec::new(),
            provider_account: Some(kernel::event::record::ProviderAccountBinding {
                provider: "house".to_owned(), account: kernel::ServerLabel::parse(account).unwrap(),
            }),
        }).unwrap();
        let returned = Payload::new(serde_json::json!({"message": {"content": []}, "calls": 0}).as_object().unwrap().clone()).unwrap();
        let mut events = vec![
            (EventKind::RunStarted, old, Some(room.clone()), Payload::empty()),
            (EventKind::RunStarted, spare, Some(room.clone()), Payload::empty()),
            (EventKind::RunStarted, other, Some(neighbour.clone()), Payload::empty()),
            (EventKind::ModelCalled, old, None, called("old")),
            (EventKind::ModelCalled, spare, None, called("spare")),
            (EventKind::ModelCalled, other, None, called("neighbour")),
            (EventKind::ModelReturned, other, None, returned.clone()),
            (EventKind::SessionOpened, RunId::CITY, Some(room.clone()), Payload::of(&kernel::event::record::SessionOpened { carried: false, from: None }).unwrap()),
        ];
        let mut live = gateway::EndpointBook::new();
        for (kind, run, addr, data) in &events {
            live.absorb(*kind, *run, addr.as_ref(), data).unwrap();
        }
        proptest::prop_assert_eq!(live.session_account(&room, "house"), None);
        let opened = serde_json::to_value(&live).unwrap();
        events.push((EventKind::ModelReturned, old, None, returned.clone()));
        live.absorb(EventKind::ModelReturned, old, None, &returned).unwrap();
        proptest::prop_assert_eq!(serde_json::to_value(&live).unwrap(), opened);
        let fresh = [
            (EventKind::RunStarted, current, Some(room.clone()), Payload::empty()),
            (EventKind::ModelCalled, current, None, called("current")),
            (EventKind::ModelReturned, current, None, returned.clone()),
        ];
        for (kind, run, addr, data) in &fresh {
            live.absorb(*kind, *run, addr.as_ref(), data).unwrap();
        }
        events.extend(fresh);
        let expected = serde_json::to_value(&live).unwrap();
        events.extend([
            (EventKind::ModelReturned, spare, Some(room.clone()), returned.clone()),
            (EventKind::ModelCalled, old, Some(room.clone()), called("old")),
            (EventKind::ModelReturned, old, Some(room.clone()), returned.clone()),
        ]);
        for step in late {
            let run = if step % 2 == 0 { old } else { spare };
            let (kind, data) = match step % 3 {
                0 => (EventKind::ModelCalled, called("late")),
                1 => (EventKind::ModelReturned, returned.clone()),
                2.. => (EventKind::RunFrozen, Payload::empty()),
            };
            let addr = if step < 6 { None } else { Some(room.clone()) };
            events.push((kind, run, addr, data));
        }
        let records: Vec<_> = (1..).zip(events).map(|(seq, (kind, run, addr, data))| {
            kernel::EventRecord::from_draft(kernel::EventDraft {
                run, t: kernel::TimeMs::new(1_000), who: "lab/room1".to_owned(), addr, kind, data, ig: false,
            }, kernel::Seq::new(seq), kernel::B3Hash::digest(b"prev"))
        }).collect();
        let mut live = gateway::EndpointBook::new();
        let mut replay = Views::new(dir.path());
        let mut restored = Views::new(dir.path());
        let cut = cut.min(records.len());
        for (index, record) in records.iter().enumerate() {
            live.absorb(record.kind(), record.run(), record.addr(), record.data()).unwrap();
            replay.apply(record).unwrap();
            if index == cut {
                restored = Views::decode(dir.path(), &restored.encode().unwrap()).unwrap();
            }
            restored.apply(record).unwrap();
        }
        restored = Views::decode(dir.path(), &restored.encode().unwrap()).unwrap();
        proptest::prop_assert_eq!(serde_json::to_value(&live).unwrap(), expected.clone());
        proptest::prop_assert_eq!(serde_json::to_value(&replay.book).unwrap(), expected.clone());
        proptest::prop_assert_eq!(serde_json::to_value(&restored.book).unwrap(), expected);
    }
}

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
