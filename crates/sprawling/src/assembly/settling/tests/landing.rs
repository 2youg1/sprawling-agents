// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use crate::assembly::fixture::*;
use crate::assembly::*;

fn speaking_signal(id: &str, room: &Address) -> collab::Signal {
    collab::Signal::new(
        collab::SignalId::parse(id).unwrap(),
        collab::SignalKind::Mention,
        "ito".to_owned(),
        room.clone(),
        kernel::Version::new(1),
        kernel::Payload::empty(),
        now_ms().unwrap(),
    )
    .unwrap()
}

/// A landing that settles halfway leaves no torn city: the lines are
/// all on the ledger, the knocks cut back to the mark, and the queue
/// holds only what the ledger says landed.
#[test]
fn a_half_settled_landing_leaves_no_torn_city() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    for who in ["ito", "hana"] {
        let room = dir.path().join("market").join(who);
        std::fs::create_dir_all(&room).unwrap();
        std::fs::write(
            room.join(city::URBANITE_FILE),
            format!(
                "# URBANITE.md

Trades in the market as {who}.
"
            ),
        )
        .unwrap();
    }
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("market/hana").unwrap();
    // One slot: the second delivery sheds, so the landing settles
    // halfway by construction rather than by luck.
    worker
        .inboxes
        .insert(room.clone(), collab::Inbox::new(1, 1));
    let knocks_mark = worker.knocks.len();
    let at = Assignment {
        addr: Address::parse("market/ito").unwrap(),
        parent: None,
        succession: None,
        session: None,
        effort: None,
        mode: runtime::Mode::Up,
    };
    let effects = vec![
        collab::SignalEffect::Enqueued(speaking_signal("s-1", &room)),
        collab::SignalEffect::Enqueued(speaking_signal("s-2", &room)),
        collab::SignalEffect::Enqueued(speaking_signal("s-3", &room)),
    ];
    let landing =
        effect::Landing::signals(effects, &Address::parse("market/ito").unwrap(), "ito").unwrap();
    let err = worker.settle(&at, RunId::CITY, landing).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::BackpressureShed);
    assert_eq!(
        worker.knocks.len(),
        knocks_mark,
        "knocks pushed by landed signals are cut back on failure"
    );
    let inbox = worker.inboxes.get(&room).unwrap();
    assert_eq!(
        inbox.pending(),
        1,
        "only the first signal landed in the queue"
    );
}

/// A shelf that files halfway is unwound: the first filing's path is
/// gone, and the ledger still carries both lines.
#[test]
fn a_half_filed_shelf_is_unwound() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = Address::parse("lab").unwrap();
    city::create_building(dir.path(), &building, city::BuildingTemplate::Minimal).unwrap();
    // The second kind's directory is a file: its filing cannot land,
    // while the first kind's files normally. The failure is
    // constructed, not lucky.
    let blocked = dir
        .path()
        .join("lab")
        .join(city::ARCHIVE_DIR)
        .join("decision");
    std::fs::create_dir_all(blocked.parent().unwrap()).unwrap();
    std::fs::write(&blocked, "in the way").unwrap();
    let at = Assignment {
        addr: Address::parse("lab").unwrap(),
        parent: None,
        succession: None,
        session: None,
        effort: None,
        mode: runtime::Mode::Up,
    };
    let effects = vec![
        collab::ArchiveEffect::Recorded {
            kind: "fact".to_owned(),
            text: "the kiln fires at cone six".to_owned(),
        },
        collab::ArchiveEffect::Recorded {
            kind: "decision".to_owned(),
            text: "fire it on friday".to_owned(),
        },
    ];
    let landing = effect::Landing::shelf(
        effects,
        dir.path(),
        &building,
        now_ms().unwrap(),
        &Address::parse("lab").unwrap(),
        "potter",
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let err = worker.settle(&at, RunId::CITY, landing).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::StorageFatal);
    // The first filing was unwound: nothing is on the shelf that the
    // history does not already carry — and the history carries both
    // lines regardless.
    let shelf = dir.path().join("lab").join(city::ARCHIVE_DIR);
    let mut leftovers = Vec::new();
    let mut dirs = vec![shelf];
    while let Some(dir_2) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir_2) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if entry.file_name() != "decision" {
                leftovers.push(path);
            }
        }
    }
    assert!(
        leftovers.is_empty(),
        "the wound-back shelf keeps filings: {leftovers:?}"
    );
}
