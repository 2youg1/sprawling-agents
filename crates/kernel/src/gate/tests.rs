// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// Every door in the roster answers something other than Allow, and
/// every Deny says all three parts. Adding a door adds a row here by
/// adding an arm to `sample`, which does not compile until it is
/// written.
#[test]
fn every_door_in_the_roster_has_an_answer_with_three_parts() {
    for door in DOORS {
        let answered = conformance::sample(door)
            .unwrap_or_else(|door| panic!("{}: the sample allowed", door.as_str()));
        let error = match &answered {
            GateOutcome::Allow => panic!("{}: the sample allowed", door.as_str()),
            GateOutcome::Deny { refusal } | GateOutcome::Ask { question: refusal } => refusal,
        };
        let parts = error
            .gate()
            .unwrap_or_else(|| panic!("{}: answer without the three parts", door.as_str()));
        assert!(!parts.rule().is_empty(), "{}: empty rule", door.as_str());
        assert!(
            !parts.violation().is_empty(),
            "{}: empty violation",
            door.as_str()
        );
        assert!(
            parts.alternative().len() > 12,
            "{}: the alternative must direct the next action",
            door.as_str()
        );
    }
}

/// The roster names each door once, and the names are the function
/// names a reader greps for.
#[test]
fn the_roster_holds_every_door_exactly_once() {
    let mut names: Vec<&str> = DOORS.iter().map(|door| door.as_str()).collect();
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), before, "a door is listed twice");
    let mut all: Vec<DoorId> = DOORS.to_vec();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), before);
}

/// Taint reaches a decision rather than a sentence in a refusal:
/// both doors that see a taint set refuse on it (C15).
#[test]
fn the_doors_that_see_taint_refuse_on_it() {
    for (door, denies) in conformance::taint_readers() {
        assert!(
            denies,
            "{}: taint changed no verdict, so C15 is a false branch there",
            door.as_str()
        );
    }
}

/// The attach door's own three answers, one test apiece: the
/// question, the loopback allow, and the network refusal.
#[test]
fn the_attach_door_asks_then_allows_loopback_then_refuses_a_network() {
    assert!(matches!(super::attach(None), GateOutcome::Ask { .. }));
    assert!(matches!(
        super::attach(Some(&super::EgressTarget::Loopback)),
        GateOutcome::Allow
    ));
    assert!(matches!(
        super::attach(Some(&super::EgressTarget::Public {
            host: "example.com".to_owned()
        })),
        GateOutcome::Deny { .. }
    ));
}
