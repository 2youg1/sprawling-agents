// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `VIEWS_FOLD_RULES` moves whenever the encoding of `Views` does
//! (`crates/sprawling/Spec.lean` §8-91).

use kernel::{Address, EventKind, RunId};

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
    for (seq, (kind, data)) in (1..).zip(records.into_iter().chain(provider_registrations())) {
        let serde_json::Value::Object(data) = data else {
            panic!("each fixture payload is an object");
        };
        views
            .apply(&view_record(Place { seq, run }, kind, &room, data))
            .unwrap();
    }
    views
}

pub(crate) fn provider_registrations() -> [(EventKind, serde_json::Value); 2] {
    [
        (
            EventKind::EndpointAttached,
            serde_json::json!({
                "name": "legacy", "base_url": "https://api.example.test/v1",
                "dialect": kernel::DialectKind::OpenAi, "models": [], "auth": "secret:providers/legacy"
            }),
        ),
        (
            EventKind::EndpointAttached,
            serde_json::json!({
                "name": "explicit", "base_url": "https://api.example.test/v1",
                "dialect": kernel::DialectKind::OpenAi, "models": [], "tuning": {"accounts": [
                    {"id": "first", "reference": "secret:providers/first"},
                    {"id": "anonymous"}
                ]}
            }),
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
