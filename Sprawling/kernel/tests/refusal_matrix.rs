// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![cfg(feature = "conformance")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::gate::{DOORS, DoorId, GateOutcome, conformance};

fn assert_complete_refusal(refusal: &kernel::AxError, door: &str) {
    let gate = refusal
        .gate()
        .unwrap_or_else(|| panic!("{door}: refusal must carry the three parts"));
    assert!(!gate.rule().is_empty(), "{door}: empty rule");
    assert!(!gate.violation().is_empty(), "{door}: empty violation");
    assert!(
        gate.alternative().len() > 12,
        "{door}: the alternative must direct the next action, not just say no"
    );
}

#[test]
fn every_door_denial_carries_a_complete_teaching_refusal() {
    for door in DOORS {
        let answered = conformance::sample(door)
            .unwrap_or_else(|allowed| panic!("{}: sample was meant to deny", allowed.as_str()));
        match answered {
            GateOutcome::Deny { refusal } => assert_complete_refusal(&refusal, door.as_str()),
            GateOutcome::Ask { question } => {
                assert_eq!(
                    question.code(),
                    &kernel::AxCode::ApprovalPending,
                    "{}: an asking door's question carries the pending code",
                    door.as_str()
                );
                assert_complete_refusal(&question, door.as_str());
            }
            GateOutcome::Allow => panic!("{}: sample was meant to deny", door.as_str()),
        }
    }
}

#[test]
fn exactly_one_door_asks_a_person() {
    let asking: Vec<DoorId> = DOORS
        .into_iter()
        .filter(|door| matches!(conformance::sample(*door), Ok(GateOutcome::Ask { .. })))
        .collect();
    assert_eq!(
        asking,
        vec![DoorId::Attach],
        "the attach door is the only door that answers Ask"
    );
}

#[test]
fn gate_code_carriers_all_point_at_gate_denied() {
    use kernel::{AxCode, Carrier, EventKind};
    for code in [
        AxCode::OutsideWriteDomain,
        AxCode::SecretEgress,
        AxCode::DiscardIrreversible,
        AxCode::DelegationDepth,
        AxCode::GateDenied,
        AxCode::TaintedAction,
        AxCode::CrossBuildingDenied,
    ] {
        assert_eq!(code.carrier(), Carrier::Event(EventKind::GateDenied));
    }
}
