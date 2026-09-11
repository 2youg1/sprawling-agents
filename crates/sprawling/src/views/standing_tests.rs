// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a page that has just opened must be able to learn from one
//! `city_view`: which room each run works in, and whether the city is
//! stopped (sprawling-SPEC section 8-52).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::tests::view_record;
use crate::views::Views;
use kernel::{Address, EventKind, RunId};

fn halt_record(seq: u64, state: &str) -> kernel::EventRecord {
    let mut data = serde_json::Map::new();
    data.insert(
        "scope".to_owned(),
        serde_json::Value::String("city".to_owned()),
    );
    data.insert(
        "state".to_owned(),
        serde_json::Value::String(state.to_owned()),
    );
    view_record(
        seq,
        RunId::CITY,
        EventKind::CityHalted,
        &Address::parse("hall").unwrap(),
        data,
    )
}

/// A page refreshed after `stop the city` used to see a city that
/// looked open, because the halt was a record it had not been sent and
/// a judgement it could not ask for.
#[test]
fn the_city_view_names_the_scopes_a_halt_shut() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    views.apply(&halt_record(1, "halted")).unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert_eq!(city.halted, vec!["city".to_owned()]);

    views.apply(&halt_record(2, "released")).unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert!(city.halted.is_empty(), "a released scope is not shut");
}

/// The run list is what a conversation is assembled from, so every run
/// has to say which room it belongs to and when it began.
#[test]
fn a_run_in_the_city_view_says_which_room_it_works_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("hall/mayor").unwrap();
    let run = RunId::from_bytes([9u8; 16]);
    let mut data = serde_json::Map::new();
    data.insert(
        "task".to_owned(),
        serde_json::Value::String("plan the week".to_owned()),
    );
    data.insert(
        "goal".to_owned(),
        serde_json::Value::String("a roadmap".to_owned()),
    );
    views
        .apply(&view_record(1, run, EventKind::RunStarted, &room, data))
        .unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    let summary = city.runs.iter().find(|r| r.run == run).unwrap();
    assert_eq!(summary.addr, Some(room));
    assert_eq!(summary.started, Some(kernel::TimeMs::new(1_000)));
}
