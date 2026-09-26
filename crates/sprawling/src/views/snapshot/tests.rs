// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `VIEWS_FOLD_RULES` moves whenever the encoding of `Views` does
//! (sprawling-SPEC 8-91).

use kernel::{Address, EventKind, RunId};

use super::*;
use crate::views::tests::view_record;

/// A view folded from records that fill the inbox, the discard bin and
/// the registry, so a field added, removed or reordered among them
/// changes the bytes.
fn fixture(city_root: &Path) -> Views {
    let mut views = Views::new(city_root);
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let tracked = format!("file:lab/room1/notes.md@{}", "ab".repeat(20));
    let records = [
        (
            EventKind::SignalEnqueued,
            serde_json::json!({"id": "sig-1", "kind": "question", "from": "lab/room2", "room": "lab/room1"}),
        ),
        (
            EventKind::FileDiscarded,
            serde_json::json!({"paths": ["file:lab/room1/notes.md"], "restoration": {"tracked": tracked}}),
        ),
        (
            EventKind::AssetArchived,
            serde_json::json!({"kind": "decision", "subject": "chose git over a second index"}),
        ),
    ];
    for (seq, (kind, data)) in (1..).zip(records) {
        let serde_json::Value::Object(data) = data else {
            panic!("each fixture payload is an object");
        };
        views
            .apply(&view_record(seq, run, kind, &room, data))
            .unwrap();
    }
    views
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
