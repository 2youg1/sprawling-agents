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

use kernel::{Locator, RunId};
use runtime::handoff::{Handoff, resume};

const SESSIONS: u32 = 1_000;

fn locator(seed: u8) -> Locator {
    Locator::parse(&format!("cas:b3-{}", format!("{seed:02x}").repeat(32))).unwrap()
}

fn handoff_of(session: u32) -> Handoff {
    Handoff::new(
        vec![locator(0), locator(1)],
        format!("session {session}"),
        "one row claimed, one row closed".to_owned(),
        "the kiln takes six hours to cool".to_owned(),
        "close the next row".to_owned(),
    )
    .unwrap()
}

#[test]
fn a_thousand_freezes_leave_the_same_thing_behind_as_the_first() {
    let first = resume(&handoff_of(0), RunId::from_bytes([1u8; 16]));
    let mut last = first.clone();
    for session in 1..SESSIONS {
        let handoff = handoff_of(session);
        last = resume(&handoff, RunId::from_bytes([1u8; 16]));

        assert_eq!(
            last.must_read.len(),
            first.must_read.len(),
            "session {session} asks its successor to read more than session 0 did"
        );
    }
    assert_eq!(
        last.must_read, first.must_read,
        "what a resume asks for is a property of the handoff, not of how many came before it"
    );
}

#[test]
fn a_resume_is_a_new_run_every_time_and_never_the_old_one() {
    let handoff = handoff_of(0);
    for session in 0..SESSIONS {
        let mut bytes = [0u8; 16];
        bytes[0] = u8::try_from(session % 256).unwrap();
        bytes[1] = u8::try_from(session / 256).unwrap();
        let seed = resume(&handoff, RunId::from_bytes(bytes));
        assert_eq!(seed.run, RunId::from_bytes(bytes));
    }
}

#[test]
fn a_handoff_that_asks_for_nothing_cannot_be_built_however_many_times_it_is_tried() {
    for _ in 0..SESSIONS {
        assert!(
            Handoff::new(
                Vec::new(),
                "overview".to_owned(),
                "progress".to_owned(),
                "context".to_owned(),
                "next".to_owned(),
            )
            .is_err()
        );
    }
}
