// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The refusal conformance matrix. Every Deny the doors can
//! produce carries the three mandatory parts, each non-empty, and the
//! alternative is directive prose (names a next action), because the
//! model is the recovery subject.

// The samples this file judges are compiled only with the conformance
// feature (`gate::conformance`, `kernel-SPEC.md` section 12.3), so the
// target is judged in that build and is empty in every other.
#![cfg(feature = "conformance")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::gate::{DOORS, DoorId, GateOutcome, conformance};

/// The matrix row: three parts present, non-empty, alternative directive.
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

/// Walks `DOORS` rather than naming doors one at a time, so a door
/// added to the enum is a door this matrix judges. The samples call the
/// real functions, so what is judged is the answer a run receives.
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

/// The YOLO rule of `kernel-SPEC.md` section 12.1 is one door wide, and
/// this is the property that says so: walking the roster, exactly one
/// door answers Ask, and it is the attach door. A second asking door
/// breaks this line rather than a reader's expectation.
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
    // The refusal codes the doors produce carry into history via
    // gate_denied (C9): the matrix cross-checks the carrier table.
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
