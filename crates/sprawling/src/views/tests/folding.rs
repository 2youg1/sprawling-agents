// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Place, view_record};
use crate::views::Views;
use kernel::{Address, EventKind, Payload, RunId};

#[test]
fn the_five_views_that_used_to_say_unavailable_answer_from_the_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([7u8; 16]);

    // The two lines exactly as `effect` writes them: the consumption
    // carries only the id and the taker, and its room is the line's addr.
    let signal = collab::Signal::new(
        kernel::event::record::SignalId::parse("sig-1").unwrap(),
        kernel::event::record::SignalKind::Thread,
        "lab/room2".to_owned(),
        room.clone(),
        kernel::Version::new(1),
        Payload::new(serde_json::Map::new()).unwrap(),
        kernel::TimeMs::new(1_000),
    )
    .unwrap();
    let line = |seq, kind, data: Payload| {
        view_record(Place { seq, run }, kind, &room, data.as_map().clone())
    };
    views
        .apply(&line(
            1,
            EventKind::SignalEnqueued,
            signal.enqueued_payload().unwrap(),
        ))
        .unwrap();

    let wire::Answer::Inbox(inbox) = views.answer(&wire::Query::InboxView { addr: room.clone() })
    else {
        panic!("InboxView answers with an inbox");
    };
    assert_eq!(inbox.waiting.len(), 1);
    assert_eq!(inbox.waiting[0].kind, "thread");

    // Taking a signal empties the row; the view never took it itself.
    views
        .apply(&line(
            2,
            EventKind::SignalConsumed,
            signal.consumed_payload("lab/room1").unwrap(),
        ))
        .unwrap();
    let wire::Answer::Inbox(inbox) = views.answer(&wire::Query::InboxView { addr: room.clone() })
    else {
        panic!("InboxView answers with an inbox");
    };
    assert!(inbox.waiting.is_empty());

    let mut discarded = serde_json::Map::new();
    discarded.insert(
        "paths".to_owned(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "file:lab/room1/notes.md".to_owned(),
        )]),
    );
    let mut way_back = serde_json::Map::new();
    way_back.insert(
        "tracked".to_owned(),
        // A real checkpoint oid, because the plan is parsed rather
        // than copied: `wave_post` writes the commit it fenced, and
        // a locator that does not parse is not a way back.
        serde_json::Value::String(format!("file:lab/room1/notes.md@{}", "ab".repeat(20))),
    );
    discarded.insert(
        "restoration".to_owned(),
        serde_json::Value::Object(way_back),
    );
    views
        .apply(&view_record(
            Place { seq: 3, run },
            EventKind::FileDiscarded,
            &room,
            discarded.clone(),
        ))
        .unwrap();
    let wire::Answer::Discards(bin) = views.answer(&wire::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert_eq!(bin.rows.len(), 1);
    // The plan travels as a plan: the interface owns the sentence,
    // and a server that composed one too would be the second place
    // that decides what a way back reads like.
    assert!(
        matches!(bin.rows[0].restoration, Some(wire::Restoration::Tracked(_))),
        "every row states its own way back: {:?}",
        bin.rows[0].restoration
    );
    assert!(!bin.rows[0].restored);

    views
        .apply(&view_record(
            Place { seq: 4, run },
            EventKind::DiscardRestored,
            &room,
            discarded.clone(),
        ))
        .unwrap();
    let wire::Answer::Discards(bin) = views.answer(&wire::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert!(
        bin.rows[0].restored,
        "a restoration closes the row it opened rather than opening a second one"
    );
    assert_eq!(bin.rows.len(), 1);

    let mut archived = serde_json::Map::new();
    archived.insert(
        "kind".to_owned(),
        serde_json::Value::String("decision".to_owned()),
    );
    archived.insert(
        "subject".to_owned(),
        serde_json::Value::String("chose git over a second index".to_owned()),
    );
    views
        .apply(&view_record(
            Place { seq: 5, run },
            EventKind::AssetArchived,
            &room,
            archived,
        ))
        .unwrap();
    let wire::Answer::Registry(registry) = views.answer(&wire::Query::RegistryView) else {
        panic!("RegistryView answers with a registry");
    };
    assert_eq!(registry.assets.len(), 1);
    assert_eq!(registry.assets[0].kind, "decision");

    let wire::Answer::Metrics(metrics) = views.answer(&wire::Query::Metrics) else {
        panic!("Metrics answers with the vital signs");
    };
    assert_eq!(metrics.events, 5, "one number no other view can derive");
    assert_eq!(metrics.signals_waiting, 0);
    assert_eq!(
        metrics.discards_outstanding, 0,
        "a restored file is not still missing"
    );

    // The archive is read from the shelves, so a city with no shelf
    // answers an empty search rather than failing to search.
    let wire::Answer::Archive(found) = views.answer(&wire::Query::ArchiveSearch {
        needle: "git".to_owned(),
    }) else {
        panic!("ArchiveSearch answers with hits");
    };
    assert_eq!(found.needle, "git");
    assert!(found.hits.is_empty());

    // A scheme this build cannot read still produces a row. Hiding a
    // discarded thing is worse than admitting the plan is unreadable,
    // and the interface has a sentence for exactly that case.
    let mut unreadable = discarded;
    unreadable.insert(
        "paths".to_owned(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "file:lab/room1/other.md".to_owned(),
        )]),
    );
    unreadable.insert(
        "restoration".to_owned(),
        serde_json::json!({ "teleported": "somewhere" }),
    );
    views
        .apply(&view_record(
            Place { seq: 6, run },
            EventKind::FileDiscarded,
            &room,
            unreadable,
        ))
        .unwrap();
    let wire::Answer::Discards(bin) = views.answer(&wire::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert_eq!(bin.rows.len(), 2, "the unreadable plan still gets a row");
    assert!(
        bin.rows
            .iter()
            .any(|row| row.path.ends_with("other.md") && row.restoration.is_none())
    );
}

/// A consumption leaves the inbox by its id, whatever room the line that
/// records it names: nothing forces a consumed line's `addr` to be the
/// room the signal was enqueued in, and `CollaborationFold` already reads
/// it that way.
#[test]
fn a_consumed_signal_leaves_the_inbox_whatever_room_its_line_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let elsewhere = Address::parse("lab/room2").unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let signal = collab::Signal::new(
        kernel::event::record::SignalId::parse("sig-1").unwrap(),
        kernel::event::record::SignalKind::Thread,
        "lab/room2".to_owned(),
        room.clone(),
        kernel::Version::new(1),
        Payload::new(serde_json::Map::new()).unwrap(),
        kernel::TimeMs::new(1_000),
    )
    .unwrap();
    views
        .apply(&view_record(
            Place { seq: 1, run },
            EventKind::SignalEnqueued,
            &room,
            signal.enqueued_payload().unwrap().as_map().clone(),
        ))
        .unwrap();
    views
        .apply(&view_record(
            Place { seq: 2, run },
            EventKind::SignalConsumed,
            &elsewhere,
            signal
                .consumed_payload("lab/room2")
                .unwrap()
                .as_map()
                .clone(),
        ))
        .unwrap();

    let wire::Answer::Inbox(inbox) = views.answer(&wire::Query::InboxView { addr: room }) else {
        panic!("InboxView answers with an inbox");
    };
    assert_eq!(inbox.waiting, Vec::new());
}

/// `as_of` is the first seq an answer does not reflect: the client holds
/// an answer read before genesis as stale once genesis lands only while
/// this holds.
#[test]
fn the_views_answer_as_of_the_first_seq_they_have_not_folded() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab").unwrap();
    assert_eq!(views.next_unfolded(), kernel::Seq::FIRST);
    views
        .apply(&view_record(
            Place {
                seq: kernel::Seq::FIRST.value(),
                run: RunId::CITY,
            },
            EventKind::CityInitialized,
            &room,
            serde_json::Map::new(),
        ))
        .unwrap();
    assert_eq!(views.next_unfolded(), kernel::Seq::FIRST.next().unwrap());
}

/// The views a rebuild hands over hold the index the rebuild read the
/// history into, so the first question after it refreshes nothing. An
/// index left behind would still answer, because `refresh` fills an empty
/// index before every read, but only by scanning the whole ledger again.
#[test]
fn the_views_a_rebuild_hands_over_hold_the_index_it_read() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::assembly::init_city(dir.path()).unwrap();
    let views = Views::rebuild(&report.ledger_dir).unwrap();
    assert_eq!(
        views
            .index
            .lock()
            .unwrap()
            .refresh(&report.ledger_dir)
            .unwrap(),
        storage::Refreshed::Unchanged
    );
}
