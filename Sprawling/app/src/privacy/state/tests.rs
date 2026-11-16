// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::fixtures::*;
use super::*;
use crate::privacy::target::{RawValue, TaskState};

const TELEMETRY: PrivacyControl = PrivacyControl::DiagnosticData;
const TRACKING: PrivacyControl = PrivacyControl::StartLaunchTracking;

fn statuses(lines: Vec<Line>) -> Result<serde_json::Value, HistoryFault> {
    History::fold(lines)
        .map(|history| serde_json::to_value(history.disclose(|_| Ok(())).unwrap()).unwrap())
}

/// Each control keeps its own stack (Privacy D51): a restore reverses the
/// top of its own control's stack whatever other controls did since, and
/// never the top of another control's stack or a lower layer of its own.
#[test]
fn restore_reverses_only_the_top_of_its_own_controls_stack() {
    let telemetry = apply(1, TELEMETRY, Snapshot::Registry(RawValue::Absent), dword(0));
    let tracking = apply(2, TRACKING, dword(1), dword(0));
    let history = [
        prepared(&telemetry),
        finished(&telemetry, Outcome::Applied),
        prepared(&tracking),
        finished(&tracking, Outcome::Applied),
    ];
    let back = restore(3, &telemetry);
    let mut accepted = history.to_vec();
    accepted.extend([prepared(&back), finished(&back, Outcome::Restored)]);
    assert!(History::fold(accepted).is_ok());

    let crossed = Intent {
        control: TRACKING,
        ..back.clone()
    };
    let mut refused = history.to_vec();
    refused.push(prepared(&crossed));
    assert!(History::fold(refused).is_err());

    let second = apply(2, TELEMETRY, dword(0), dword(1));
    let lower = restore(3, &telemetry);
    assert!(
        History::fold(vec![
            prepared(&telemetry),
            finished(&telemetry, Outcome::Applied),
            prepared(&second),
            finished(&second, Outcome::Applied),
            prepared(&lower),
        ])
        .is_err()
    );
}

/// Only a matched apply owns: a rolled-back, not-applied or abandoned
/// operation leaves nothing to restore, and a settlement of `Applied`
/// owns as a receipt of `Applied` does.
#[test]
fn only_a_matched_apply_takes_ownership() {
    for (end, owns) in [
        (vec![Outcome::RolledBack], false),
        (vec![Outcome::NotApplied], false),
        (vec![Outcome::Unknown], false),
        (vec![Outcome::Applied], true),
    ] {
        let intent = apply(1, TELEMETRY, dword(5), dword(0));
        let mut lines = vec![prepared(&intent)];
        lines.extend(end.iter().map(|outcome| finished(&intent, *outcome)));
        lines.push(prepared(&restore(2, &intent)));
        assert_eq!(History::fold(lines).is_ok(), owns, "{end:?}");
    }
    for (settlement, owns) in [
        (PrivacySettlement::Abandoned, false),
        (PrivacySettlement::NotApplied, false),
        (PrivacySettlement::Applied, true),
    ] {
        let intent = apply(1, TELEMETRY, dword(5), dword(0));
        let lines = vec![
            prepared(&intent),
            finished(&intent, Outcome::Unknown),
            reconciled(&intent, settlement),
            prepared(&restore(2, &intent)),
        ];
        assert_eq!(History::fold(lines).is_ok(), owns, "{settlement:?}");
    }
}

/// An unknown readback keeps the operation open: status says
/// unresolved, no second receipt and no new operation is accepted, and
/// only a reconciliation of that operation ends it.
#[test]
fn an_unknown_operation_ends_only_by_its_reconciliation() {
    let intent = apply(1, TELEMETRY, dword(5), dword(0));
    let unknown = vec![prepared(&intent), finished(&intent, Outcome::Unknown)];
    assert_eq!(
        statuses(unknown.clone()).unwrap(),
        serde_json::json!([{ "operation": 1, "outcome": "unresolved" }])
    );
    let next = apply(2, TRACKING, dword(1), dword(0));
    let other = apply(2, TELEMETRY, dword(5), dword(0));
    for refused in [
        finished(&intent, Outcome::Applied),
        prepared(&next),
        reconciled(&other, PrivacySettlement::Abandoned),
    ] {
        let mut lines = unknown.clone();
        lines.push(refused);
        assert!(History::fold(lines).is_err());
    }
    let mut settled = unknown;
    settled.extend([
        reconciled(&intent, PrivacySettlement::Abandoned),
        prepared(&next),
    ]);
    assert_eq!(
        statuses(settled).unwrap(),
        serde_json::json!([
            { "operation": 1, "outcome": { "reconciled": "abandoned" } },
            { "operation": 2, "outcome": "unresolved" }
        ])
    );
    assert!(History::fold(vec![reconciled(&intent, PrivacySettlement::Applied)]).is_err());
}

fn snapshots() -> impl proptest::strategy::Strategy<Value = Snapshot> {
    use proptest::prelude::*;
    prop_oneof![
        Just(Snapshot::Registry(RawValue::Absent)),
        (any::<u32>(), proptest::collection::vec(any::<u8>(), 0..512))
            .prop_map(|(kind, bytes)| Snapshot::Registry(RawValue::Present { kind, bytes })),
        Just(Snapshot::Task(TaskState::Absent)),
        any::<[u8; 32]>().prop_map(|definition_sha256| {
            Snapshot::Task(TaskState::Enabled { definition_sha256 })
        }),
        any::<[u8; 32]>().prop_map(|definition_sha256| {
            Snapshot::Task(TaskState::Disabled { definition_sha256 })
        }),
    ]
}

proptest::proptest! {
    /// Original values come back exactly as they were written: type
    /// code, every byte, absence, and a task's definition digest.
    #[test]
    fn intents_roundtrip_without_normalization(
        original in snapshots(),
        modified in snapshots(),
        key_existed in proptest::bool::ANY,
    ) {
        let before = Intent { key_existed, ..apply(1, TELEMETRY, original, modified) };
        let decoded: Line = serde_json::from_slice(&bytes(&[prepared(&before)])).unwrap();
        let Event::Prepared { intent } = decoded.event else {
            return Err(proptest::test_runner::TestCaseError::fail("not a prepared line"));
        };
        proptest::prop_assert_eq!(before, intent);
    }
}
