// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! Guess versus decision, through the production door.

use super::listing::{listing, spent_of};
use super::plan::{DEFAULT_MODE, ENDED_ROWS, Field, Plan};

use crate::app::Snapshot;
use crate::lang::Lang;
use crate::phase::Phase;

/// The one visual rule this page is built on: a word the city
/// guessed and a word the person set may not be drawn alike.
#[test]
fn a_guess_and_a_decision_are_drawn_differently() {
    let mut plan = Plan {
        room: "lab/parser".to_owned(),
        mode: DEFAULT_MODE.to_owned(),
        effort: "medium".to_owned(),
        chosen: Vec::new(),
    };
    for field in Field::ALL {
        assert_eq!(plan.ink(field), "guess");
    }
    plan.choose(Field::Mode, "sc".to_owned());
    assert_eq!(plan.ink(Field::Mode), "chosen");
    assert_eq!(plan.ink(Field::Room), "guess");
    assert_eq!(plan.ink(Field::Effort), "guess");
}

/// Agreeing with a guess is a decision. Without this, a person who
/// opened the word, read it and kept it would still be shown the
/// city's own dotted guess.
#[test]
fn keeping_the_guessed_value_still_counts_as_choosing_it() {
    let mut plan = Plan {
        mode: DEFAULT_MODE.to_owned(),
        ..Plan::default()
    };
    plan.choose(Field::Mode, DEFAULT_MODE.to_owned());
    assert_eq!(plan.mode, DEFAULT_MODE);
    assert_eq!(plan.ink(Field::Mode), "chosen");
}

/// A city with nothing in it guesses no room, which opens that one
/// control instead of inventing a building name.
#[test]
fn an_empty_city_guesses_no_room() {
    let plan = Plan::guessed(&Snapshot::new(), "medium");
    assert!(plan.room.is_empty());
    assert_eq!(plan.mode, DEFAULT_MODE);
    assert_eq!(plan.effort, "medium");
}

/// Absent money is not zero money. A subscription settles no amount,
/// and `$0.00` would be a figure nobody sent.
#[test]
fn an_unpriced_session_does_not_read_as_a_free_one() {
    assert_eq!(spent_of(Lang::En, None), "not priced");
    assert_ne!(spent_of(Lang::En, None), "$0.00");
    assert_eq!(
        spent_of(Lang::En, Some(channels::UsdMicros::new(420_000))),
        "$0.42"
    );
}

/// What waits on a person is at the top of the first list, which is
/// the reason the page was opened.
#[test]
fn what_waits_on_a_person_leads_the_list() {
    let snapshot = crate::app::seated(&[
        (Some("lab/one"), Phase::Running, 1),
        (Some("lab/two"), Phase::Waiting, 3),
        (Some("lab/three"), Phase::Running, 5),
        (Some("lab/gone"), Phase::Frozen, 7),
    ]);
    let (in_flight, ended) = listing(&snapshot);
    assert_eq!(in_flight.len(), 3);
    assert_eq!(in_flight[0].addr.as_str(), "lab/two");
    assert_eq!(in_flight[0].phase, Phase::Waiting);
    // Newest first inside a phase.
    assert_eq!(in_flight[1].addr.as_str(), "lab/three");
    assert_eq!(ended.len(), 1);
    assert_eq!(ended[0].addr.as_str(), "lab/gone");
}

/// A page opened to act on does not become a page to scroll.
#[test]
fn the_finished_list_stops_at_eight() {
    let rooms: Vec<String> = (0..20).map(|index| format!("lab/room{index}")).collect();
    let seats: Vec<(Option<&str>, Phase, u64)> = rooms
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                Some(name.as_str()),
                Phase::Frozen,
                u64::try_from(index)
                    .unwrap_or_default()
                    .saturating_mul(2)
                    .saturating_add(1),
            )
        })
        .collect();
    let snapshot = crate::app::seated(&seats);
    let (in_flight, ended) = listing(&snapshot);
    assert!(in_flight.is_empty());
    assert_eq!(ended.len(), ENDED_ROWS);
    // The newest eight, not the first eight.
    assert_eq!(ended[0].addr.as_str(), "lab/room19");
}

/// A run this client cannot name has no row, because a row that
/// cannot be opened spends a reader's click on nothing.
#[test]
fn a_run_with_no_address_gets_no_row() {
    let snapshot = crate::app::seated(&[
        (Some("lab/named"), Phase::Running, 1),
        (None, Phase::Running, 3),
    ]);
    assert_eq!(snapshot.runs().count(), 2, "both runs are folded");
    let (in_flight, _) = listing(&snapshot);
    assert_eq!(in_flight.len(), 1);
    assert_eq!(in_flight[0].addr.as_str(), "lab/named");
}
