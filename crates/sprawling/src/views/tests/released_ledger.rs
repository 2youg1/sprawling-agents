// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Place, view_record};
use crate::assembly::*;
use crate::views::Views;
use kernel::{Address, EventKind, Payload, RunId};

/// The prompt a run was frozen with is read out of the ledger and the
/// store after the views are released: a line and an object that land
/// between `prepare` and `finish` are both ones the answer shows.
#[test]
fn the_prompt_a_run_was_told_is_read_after_the_views_are_released() {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let room = Address::parse("lab/room1").unwrap();
    let told = b"the city says";
    let hash = kernel::B3Hash::digest(told);
    let data = serde_json::json!({ "segments": [
        { "slot": "city", "hash": hash, "len": told.len(), "sources": [] }
    ] });
    let data = data.as_object().unwrap().clone();
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, kernel::TimeMs::new(9))
        .unwrap()
        .0;
    let mut views = Views::new(dir.path());
    let at = ledger.position().value();
    views
        .apply(&view_record(
            Place { seq: at, run },
            EventKind::PromptAssembled,
            &room,
            data.clone(),
        ))
        .unwrap();

    let prepared = views.prepare(&wire::Query::Prefix { run });
    ledger
        .append(kernel::EventDraft {
            run,
            t: kernel::TimeMs::new(1_000),
            who: "lab/room1".to_owned(),
            addr: Some(room),
            kind: EventKind::PromptAssembled,
            data: Payload::new(data).unwrap(),
            ig: false,
        })
        .unwrap();
    drop(ledger);
    storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas())
        .unwrap()
        .put(told)
        .unwrap();

    assert_eq!(
        prepared.finish(),
        wire::Answer::Prefix(Box::new(wire::PrefixAnswer {
            run,
            segments: vec![wire::PrefixSegment {
                slot: wire::PrefixSlot::City,
                hash,
                bytes: 13,
                text: "the city says".to_owned(),
                stored: true,
                sources: Vec::new(),
            }],
        }))
    );
}

/// A plan nobody has read yet is read after the views are released, so
/// the building page shows the table as it stands at `finish`.
#[test]
fn a_plan_nobody_has_read_yet_is_read_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(&building).unwrap();
    let query = wire::Query::BuildingView {
        addr: Address::parse("lab").unwrap(),
    };
    let prepared = Views::new(dir.path()).prepare(&query);
    std::fs::write(
        building.join("Roadmap.md"),
        "# Roadmap\n\n| # | Item | Weight | Needs | Status | Evidence |\n\
         |---|---|---|---|---|---|\n\
         | 1 | wired | 1 |  | not started |  |\n",
    )
    .unwrap();

    assert_eq!(prepared.finish(), Views::new(dir.path()).answer(&query));
}

/// The history, a named range, one run's transcript, its rounds and its
/// evidence are read out of the ledger after the views are released: a
/// line appended between `prepare` and `finish` is one each answer shows.
#[test]
fn the_ledger_readers_read_after_the_views_are_released() {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, kernel::TimeMs::new(9))
        .unwrap()
        .0;
    let appended_at = ledger.position();
    let mut views = Views::new(dir.path());
    let queries = [
        wire::Query::History {
            before: None,
            limit: 50,
        },
        wire::Query::HistoryRange {
            from: appended_at,
            to: appended_at,
            limit: 50,
        },
        wire::Query::RunHistory {
            run,
            before: None,
            limit: 50,
        },
        wire::Query::Rounds { run },
        wire::Query::Evidence { run },
    ];
    let prepared: Vec<_> = queries.iter().map(|query| views.prepare(query)).collect();
    let data = serde_json::json!({ "segments": [] });
    ledger
        .append(kernel::EventDraft {
            run,
            t: kernel::TimeMs::new(1_000),
            who: "lab/room1".to_owned(),
            addr: Some(Address::parse("lab/room1").unwrap()),
            kind: EventKind::PromptAssembled,
            data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
            ig: false,
        })
        .unwrap();
    drop(ledger);

    let read: Vec<_> = prepared
        .into_iter()
        .map(|prepared| prepared.finish())
        .collect();
    let fresh: Vec<_> = queries.iter().map(|query| views.answer(query)).collect();
    assert_eq!(read, fresh);
}

#[test]
fn the_mcp_health_page_reads_its_servers_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let views = Views::new(dir.path());
    let prepared = views.prepare(&wire::Query::McpHealth { addr: room.clone() });
    let config = city::config_path(dir.path(), &room, city::Layer::City).unwrap();
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(
        &config,
        "[[mcp]]\nlabel = \"apps\"\ncommand = \"sprawling-no-such-server\"\n",
    )
    .unwrap();
    let wire::Answer::McpHealth(answer) = prepared.finish() else {
        panic!("not an MCP health answer");
    };
    let labels: Vec<_> = answer
        .servers
        .iter()
        .map(|server| server.label.as_str().to_owned())
        .collect();
    assert_eq!(labels, vec!["apps".to_owned()]);
}

/// A reader that panics while it holds the ledger index leaves the lock
/// poisoned for the life of the process. The next reader rebuilds the
/// index from the segments instead of answering this and every later
/// history question with an empty page.
#[test]
fn a_poisoned_ledger_index_is_rebuilt_rather_than_read_as_an_empty_history() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut views = Views::new(dir.path());
    let query = wire::Query::History {
        before: None,
        limit: 50,
    };
    let before = views.answer(&query);
    let wire::Answer::History(page) = &before else {
        panic!("History answers with a page: {before:?}");
    };
    assert!(!page.records.is_empty(), "genesis is in the history");
    let index = std::sync::Arc::clone(&views.index);
    let died = std::thread::spawn(move || {
        let _held = index.lock().unwrap();
        panic!("a reader dies holding the index");
    })
    .join();
    assert!(died.is_err() && views.index.is_poisoned());

    assert_eq!(views.answer(&query), before);
    assert!(!views.index.is_poisoned(), "the rebuild clears the poison");
}
