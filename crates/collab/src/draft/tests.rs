// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn draft(seen: u64) -> Draft {
    Draft::new(
        "lab/room1".to_owned(),
        Address::parse("lab").unwrap(),
        Version::new(seen),
        Payload::empty(),
    )
}

#[test]
fn a_draft_written_against_the_room_as_it_stands_goes_through() {
    let mut desk = Drafts::new();
    assert_eq!(
        desk.submit(&draft(4), Version::new(4), 0),
        Submission::Delivered
    );
    assert_eq!(desk.holds(&draft(4)), 0);
}

#[test]
fn a_draft_written_against_an_older_room_is_held_with_all_four_ways_back() {
    let mut desk = Drafts::new();
    let Submission::Held { token, holds } = desk.submit(&draft(3), Version::new(5), 0) else {
        panic!("a stale draft is held");
    };
    assert_eq!(holds, 1);
    assert_eq!(token.shown(), Version::new(5));

    let payload = desk.held_payload(&draft(3), Version::new(5)).unwrap();
    let returns = payload.as_map().get("returns").unwrap().as_array().unwrap();
    assert_eq!(returns.len(), 4, "the choice belongs to the author");
    assert_eq!(
        payload.as_map().get("seen").and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        payload.as_map().get("current").and_then(Value::as_u64),
        Some(5)
    );
}

#[test]
fn sending_anyway_needs_the_room_to_be_where_it_was_when_the_hold_was_shown() {
    let mut desk = Drafts::new();
    desk.submit(&draft(3), Version::new(5), 0);

    // The room moved again while the author was deciding.
    let voided = desk.resolve(&draft(3), Return::ForceInformed, Version::new(6), 0);
    assert!(matches!(voided, Resolution::TokenVoid { .. }));

    // Shown again at six, the bypass is a confirmation of what the
    // author has now actually been told.
    assert_eq!(
        desk.resolve(&draft(3), Return::ForceInformed, Version::new(6), 0),
        Resolution::Delivered
    );
}

#[test]
fn a_token_does_not_outlive_the_turn_it_was_issued_in() {
    let mut desk = Drafts::new();
    desk.submit(&draft(3), Version::new(5), 7);
    let stale = desk.resolve(&draft(3), Return::ForceInformed, Version::new(5), 8);
    assert!(
        matches!(stale, Resolution::TokenVoid { .. }),
        "a bypass carried into the next turn confirms nothing"
    );
}

#[test]
fn a_bypass_cannot_be_brought_along_in_advance() {
    let mut desk = Drafts::new();
    // No hold was ever shown: nothing to confirm, so nothing passes.
    let unearned = desk.resolve(&draft(3), Return::ForceInformed, Version::new(5), 0);
    assert!(matches!(unearned, Resolution::TokenVoid { .. }));
}

#[test]
fn resubmitting_takes_the_same_door_and_can_be_held_again() {
    let mut desk = Drafts::new();
    desk.submit(&draft(3), Version::new(5), 0);
    let again = desk.resolve(&draft(3), Return::SendAsIs, Version::new(6), 0);
    assert!(matches!(again, Resolution::Held { holds: 2, .. }));

    // A rewrite that matches the room goes through and clears the count.
    assert_eq!(
        desk.resolve(&draft(6), Return::Rewrite, Version::new(6), 0),
        Resolution::Delivered
    );
    assert_eq!(desk.holds(&draft(6)), 0);
}

#[test]
fn withdrawing_ends_it_and_forgets_the_count() {
    let mut desk = Drafts::new();
    desk.submit(&draft(3), Version::new(5), 0);
    assert_eq!(
        desk.resolve(&draft(3), Return::Withdraw, Version::new(5), 0),
        Resolution::Withdrawn
    );
    assert_eq!(desk.holds(&draft(3)), 0);
    let record = desk.resolved_payload(&draft(3), Return::Withdraw).unwrap();
    assert_eq!(
        record.as_map().get("return").and_then(Value::as_str),
        Some("withdraw")
    );
}

#[test]
fn a_livelock_stops_burning_turns_and_asks_the_owner() {
    let mut desk = Drafts::new();
    let mut last = Submission::Delivered;
    // The room keeps moving under the author, which is the livelock.
    for room in (5u64..).take(usize::try_from(DRAFT_HELD_ESCALATE).unwrap_or(0)) {
        last = desk.submit(&draft(3), Version::new(room), 0);
    }
    assert_eq!(
        last,
        Submission::Escalated {
            holds: DRAFT_HELD_ESCALATE
        },
        "two agents knocking each other back do not loop forever"
    );
}
