// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every harness line reads back whole, and two written lines pin the
//! keys the Ledger spells.

use super::*;
use crate::event::Payload;

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

#[test]
fn a_harness_line_reads_back_whole() {
    let reports = [
        HarnessReported::Said {
            text: "done".to_owned(),
        },
        HarnessReported::Thought {
            text: "read the file first".to_owned(),
        },
        HarnessReported::ToolCall {
            call: "c1".to_owned(),
            title: "Read README.md".to_owned(),
            kind: "read".to_owned(),
        },
        HarnessReported::ToolCallStatus {
            call: "c1".to_owned(),
            status: "completed".to_owned(),
        },
        HarnessReported::Other {
            variant: "plan".to_owned(),
        },
        HarnessReported::PermissionAsked {
            title: "Edit src/lib.rs".to_owned(),
            options: vec![
                HarnessPermit {
                    id: "yes".to_owned(),
                    name: "Allow".to_owned(),
                    kind: HarnessPermitKind::AllowOnce,
                },
                HarnessPermit {
                    id: "never".to_owned(),
                    name: "Reject always".to_owned(),
                    kind: HarnessPermitKind::RejectAlways,
                },
            ],
        },
        HarnessReported::PermissionAnswered { chosen: None },
    ];
    for report in reports {
        assert_eq!(
            Payload::of(&report)
                .unwrap()
                .read::<HarnessReported>()
                .unwrap(),
            report
        );
    }
    let answer = HarnessAnswered {
        stop: HarnessStop::EndTurn,
        text: "done".to_owned(),
    };
    assert_eq!(
        Payload::of(&answer)
            .unwrap()
            .read::<HarnessAnswered>()
            .unwrap(),
        answer
    );

    let said = HarnessReported::Said {
        text: "done".to_owned(),
    };
    assert_eq!(
        bytes(&Payload::of(&said).unwrap()),
        r#"{"report":"said","text":"done"}"#
    );
    assert_eq!(
        bytes(&Payload::of(&answer).unwrap()),
        r#"{"stop":"end_turn","text":"done"}"#
    );
}
