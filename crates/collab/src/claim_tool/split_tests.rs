// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::Tool;
use kernel::spine::set_roadmap_status;

use super::tests::{PLAN, call, desk, node};
use super::*;

#[test]
fn splitting_grows_the_plan_and_the_run_stops_holding_the_branch() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "action": "split",
            "node": "1",
            "parts": [{"item": "run the cable", "weight": 3}, "test the element"]
        })))
        .unwrap();
    assert_eq!(
        outcome
            .result
            .as_map()
            .get("children")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    let unfinished = outcome.result.as_map().get("unfinished").cloned();
    assert_eq!(
        unfinished,
        Some(Value::from(2)),
        "the split reports what is left"
    );
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1.1 | run the cable | 3 |  | Not started |  |"));
    assert!(
        text.contains("| 1.2 | test the element | 1 |  | Not started |  |"),
        "a bare string is a child of weight one"
    );
    assert!(
        shared.lock().unwrap().holding().is_none(),
        "the work it took is now several pieces; it takes one of them next"
    );
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects[1].kind(), kernel::EventKind::RoadmapSplit);
}

/// The eighth finding of `tools/adversary/Spec.lean`: a split of a row
/// nobody held was answered as done and then dropped at landing. The
/// desk now refuses it at the call (collab D6), queues nothing and leaves
/// the plan as the claim left it, and the refusal names what the run
/// holds and the claim that would make the split possible.
#[test]
fn a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call() {
    let split = |id: &str| {
        call(serde_json::json!({
            "action": "split", "node": id, "parts": ["run the cable", "test the element"]
        }))
    };
    // What a desk queued and wrote, and what it still holds.
    let left = |shared: &Arc<Mutex<ClaimDesk>>| {
        let mut desk = shared.lock().unwrap();
        let kinds: Vec<_> = desk.take_effects().iter().map(ClaimEffect::kind).collect();
        (
            kinds,
            desk.roadmap().map(str::to_owned),
            desk.holding().cloned(),
        )
    };
    let idle = desk();
    let unheld = ClaimTool::new(Arc::clone(&idle))
        .unwrap()
        .invoke(&split("1"))
        .err();
    let busy = desk();
    let tool = ClaimTool::new(Arc::clone(&busy)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let elsewhere = tool.invoke(&split("3")).err();
    let claimed = set_roadmap_status(PLAN, &node("1"), RoadmapStatus::InProgress, None).unwrap();

    assert_eq!(
        (unheld, left(&idle), elsewhere, left(&busy)),
        (
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "split a plan node",
                    "this run holds nothing, so it cannot split 1",
                )
                .with_recovery("claim 1 first; a row is divided by the run that holds it")
            ),
            (Vec::new(), None, None),
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "split a plan node",
                    "this run holds 1, not 3",
                )
                .with_recovery("split 1, or put it down and claim 3 first")
            ),
            (
                vec![kernel::EventKind::RoadmapClaimed],
                Some(claimed),
                Some(node("1"))
            )
        ),
        "a split the desk cannot land is refused before anything is queued"
    );
}
