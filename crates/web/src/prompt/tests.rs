// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Given, Sighting, given, glimpse};
use channels::{EventKind, EventRecord, RunId, Seq};
use serde_json::json;

fn record(seq: u64, run: RunId, kind: EventKind, data: serde_json::Value) -> EventRecord {
    let line = json!({
        "v": 1,
        "run": run,
        "seq": Seq::new(seq),
        "prev": "0".repeat(64),
        "t": 1_000_u64,
        "who": "resident",
        "kind": kind,
        "data": data,
    });
    EventRecord::parse_line(line.to_string().as_bytes()).unwrap()
}

fn started(seq: u64, run: RunId, skills: &[(&str, &str)]) -> EventRecord {
    let pins: Vec<serde_json::Value> = skills
        .iter()
        .map(|(name, hash)| json!({ "name": name, "hash": hash }))
        .collect();
    record(
        seq,
        run,
        EventKind::RunStarted,
        json!({ "task": "close the loop", "skills": pins }),
    )
}

fn assembled(seq: u64, run: RunId, city: &str, bytes: u64) -> EventRecord {
    record(
        seq,
        run,
        EventKind::PromptAssembled,
        json!({
            "segments": [
                { "slot": "city", "hash": city, "len": bytes },
                { "slot": "building", "hash": "b".repeat(64), "len": 20_u64 },
                { "slot": "resident", "hash": "c".repeat(64), "len": 30_u64 },
                { "slot": "run", "hash": "d".repeat(64), "len": 40_u64 },
            ]
        }),
    )
}

fn run(byte: u8) -> RunId {
    RunId::from_bytes([byte; 16])
}

/// The card's closing assertion: change one byte of a skill and the
/// page says so. The hash the city recorded moves, and the run that
/// was given the new bytes reports what the old ones were.
#[test]
fn a_skill_whose_bytes_moved_is_named_against_what_it_was() {
    let old = "a".repeat(64);
    let new = "e".repeat(64);
    let records = vec![
        started(1, run(1), &[("review", &old)]),
        started(2, run(2), &[("review", &new)]),
    ];
    let held = given(&records, run(2));
    assert_eq!(held.skills.len(), 1);
    assert_eq!(held.skills[0].sighting, Sighting::Changed { was: old });
    assert!(held.disturbed(), "the tab itself has to say so");
}

/// An unchanged shelf must not read as a disturbance: a warning that
/// fires every time is a warning nobody reads.
#[test]
fn a_shelf_nobody_touched_reports_no_change() {
    let same = "a".repeat(64);
    let records = vec![
        started(1, run(1), &[("review", &same)]),
        started(2, run(2), &[("review", &same)]),
    ];
    let held = given(&records, run(2));
    assert_eq!(held.skills[0].sighting, Sighting::Same);
    assert!(!held.disturbed());
}

/// Which earlier reading to compare against is decided by `seq`, not
/// by the order the slice arrived in: backfill and the live stream
/// reach a page as two deliveries, and picking the last one handed
/// over would compare against whichever request answered last.
#[test]
fn the_comparison_is_the_newest_earlier_reading_whatever_order_it_arrived_in() {
    let first = "a".repeat(64);
    let middle = "b".repeat(64);
    let now = "c".repeat(64);
    let records = vec![
        started(3, run(3), &[("review", &now)]),
        started(1, run(1), &[("review", &first)]),
        started(2, run(2), &[("review", &middle)]),
    ];
    assert_eq!(
        given(&records, run(3)).skills[0].sighting,
        Sighting::Changed { was: middle }
    );
}

/// The first time this city sees a skill there is nothing to compare
/// against, and saying "unchanged" there would be a claim the city
/// cannot support.
#[test]
fn a_skill_this_city_has_not_recorded_before_says_so() {
    let records = vec![started(1, run(1), &[("review", &"a".repeat(64))])];
    assert_eq!(given(&records, run(1)).skills[0].sighting, Sighting::First);
}

/// The hashes are read out of the record, never recomputed here: a
/// page that hashed the blocks again would be a second authority on
/// what the run was given.
#[test]
fn the_blocks_are_the_newest_assembly_and_the_turn_is_how_many() {
    let first = "a".repeat(64);
    let latest = "f".repeat(64);
    let records = vec![
        started(1, run(1), &[]),
        assembled(2, run(1), &first, 10),
        assembled(3, run(1), &latest, 11),
    ];
    let held = given(&records, run(1));
    assert_eq!(held.turn, 2, "one assembly opens each turn");
    assert_eq!(held.blocks.len(), 4);
    assert_eq!(held.blocks[0].hash, latest);
    assert_eq!(held.bytes(), 11 + 20 + 30 + 40);
}

/// Another session's records are another session's. Folding them in
/// would draw one run's prompt on another run's page.
#[test]
fn only_this_run_is_read() {
    let records = vec![
        assembled(1, run(1), &"a".repeat(64), 10),
        started(2, run(2), &[]),
    ];
    assert_eq!(given(&records, run(2)), Given::default());
}

/// A run that has started and not yet assembled anything has no
/// prompt to show, which the page states rather than drawing an
/// empty frame.
#[test]
fn a_session_before_its_first_turn_has_nothing_to_show() {
    let records = vec![started(1, run(1), &[])];
    assert!(given(&records, run(1)).blocks.is_empty());
}

#[test]
fn a_hash_is_glimpsed_rather_than_walled() {
    assert_eq!(glimpse(&"a".repeat(64)), "aaaaaaaaaaaa");
    assert_eq!(glimpse("short"), "short");
}
