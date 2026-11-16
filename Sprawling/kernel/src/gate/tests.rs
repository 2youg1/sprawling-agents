// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

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

#[test]
fn the_doors_that_see_taint_refuse_on_it() {
    assert_eq!(
        conformance::taint_readers(),
        [DoorId::Undoable, DoorId::Discard, DoorId::Command]
            .into_iter()
            .map(|door| (door, true))
            .collect(),
        "a door whose verdict taint does not change makes C15 a false branch there"
    );
}

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

#[test]
fn each_private_range_is_private_on_its_own() {
    for host in [
        "10.1.2.3",
        "192.168.0.9",
        "172.16.0.1",
        "172.31.255.1",
        "169.254.1.1",
    ] {
        assert_eq!(target_of(host), EgressTarget::Private, "{host}");
    }
    for host in [
        "11.1.2.3",
        "192.169.0.9",
        "193.168.0.9",
        "172.32.0.1",
        "173.16.0.1",
        "169.255.1.1",
        "170.254.1.1",
    ] {
        assert_eq!(
            target_of(host),
            EgressTarget::Public {
                host: host.to_owned()
            },
            "{host}"
        );
    }
}

#[test]
fn an_allowlist_is_empty_only_when_nothing_was_entered() {
    assert!(EgressAllowlist::new(Vec::new()).is_empty());
    assert!(!EgressAllowlist::new(vec!["example.com".to_owned()]).is_empty());
}

#[test]
fn a_refused_attachment_names_the_far_end_by_its_class() {
    for (target, subject) in [
        (EgressTarget::Private, "a host on the same private network"),
        (
            EgressTarget::Public {
                host: "browser.example.test".to_owned(),
            },
            "browser.example.test",
        ),
    ] {
        let GateOutcome::Deny { refusal } = attach(Some(&target)) else {
            panic!("{target:?}: a non-loopback browser address is refused");
        };
        assert_eq!(refusal.subject(), subject);
    }
}
