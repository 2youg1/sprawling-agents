// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The window and the wording, through the production door.

use channels::{EventKind, EventRecord, Seq};

use super::describe::describe_in;
use super::feed::{Feed, WINDOW};
use channels::{B3Hash, EventDraft, Payload, RunId, TimeMs};

fn record(seq: u64, kind: EventKind, who: &str) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([1u8; 16]),
            t: TimeMs::new(seq),
            who: who.to_owned(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        },
        Seq::new(seq),
        B3Hash::digest(b"prev"),
    )
}

#[test]
fn the_window_is_bounded_and_says_what_it_dropped() {
    let mut feed = Feed::new();
    let overflow = u64::try_from(WINDOW).unwrap() + 50;
    for seq in 1..=overflow {
        feed.push(&record(seq, EventKind::ToolResult, "resident"));
    }
    assert_eq!(feed.lines().len(), WINDOW);
    assert_eq!(feed.dropped(), 50);
    // The oldest line still on screen is the 51st, and the history has
    // all of them - that is what ledger_view is for.
    assert_eq!(feed.lines()[0].seq, Seq::new(51));
}

#[test]
fn a_reader_who_scrolled_back_is_not_yanked_to_the_bottom() {
    let mut feed = Feed::new();
    assert!(feed.push(&record(1, EventKind::ToolCalled, "r")));
    feed.stop_following();
    assert!(!feed.push(&record(2, EventKind::ToolResult, "r")));
    assert!(!feed.push(&record(3, EventKind::ToolResult, "r")));
    // The lines still arrive; only the scroll is withheld.
    assert_eq!(feed.lines().len(), 3);
    feed.follow();
    assert!(feed.push(&record(4, EventKind::ToolResult, "r")));
}

#[test]
fn a_persons_steer_is_marked_and_an_agents_is_not() {
    let mut feed = Feed::new();
    feed.push(&record(1, EventKind::SteerReceived, "user"));
    feed.push(&record(2, EventKind::SteerReceived, "@7 (auditor)"));
    assert!(feed.lines()[0].from_person);
    assert!(!feed.lines()[1].from_person);
}

#[test]
fn a_line_describes_the_shape_and_not_the_bytes() {
    let text = describe_in(
        crate::lang::Lang::En,
        &record(1, EventKind::ToolCalled, "resident"),
    );
    assert_eq!(text, "resident calls a tool");
    // An unmodelled kind still produces a line rather than a blank,
    // and the kind keeps the spelling the Ledger uses.
    let fallback = describe_in(
        crate::lang::Lang::Zh,
        &record(2, EventKind::PromptAssembled, "resident"),
    );
    assert!(fallback.contains("resident"));
    assert!(fallback.contains("PromptAssembled"));
}
