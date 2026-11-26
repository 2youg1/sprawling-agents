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
    assert_eq!(effects[1].kind(), Some(kernel::EventKind::RoadmapSplit));
}

#[test]
fn a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call() {
    let split = |id: &str| {
        call(serde_json::json!({
            "action": "split", "node": id, "parts": ["run the cable", "test the element"]
        }))
    };
    let left = |shared: &Arc<Mutex<ClaimDesk>>| {
        let mut desk = shared.lock().unwrap();
        let kinds: Vec<_> = desk
            .take_effects()
            .iter()
            .filter_map(ClaimEffect::kind)
            .collect();
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

const EMPTY_PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
";

fn empty_desk(room: &str) -> Arc<Mutex<ClaimDesk>> {
    Arc::new(Mutex::new(ClaimDesk::new(
        "mayor@hall.1".to_owned(),
        Address::parse(room).unwrap(),
        EMPTY_PLAN.to_owned(),
        Booking::new(|_| Ok(())),
    )))
}

#[test]
fn the_mayor_writes_the_first_line_of_an_empty_plan() {
    let shared = empty_desk(kernel::consts_policy::HALL_MAYOR);
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let added = tool
        .invoke(&call(serde_json::json!({
            "action": "add", "parts": [{"item": "survey the river", "weight": 2}]
        })))
        .ok()
        .map(|outcome| outcome.result.as_map().get("nodes").cloned());
    let text = shared.lock().unwrap().roadmap().map(str::to_owned);
    let whole = text.as_deref().and_then(|text| {
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
            return None;
        };
        let tree = PlanTree::build(rows).ok()?;
        let leaves: Vec<kernel::Share> = tree
            .nodes()
            .filter(|node| node.is_leaf())
            .map(|node| node.share)
            .collect();
        Some(kernel::share::gather(&leaves))
    });
    assert_eq!(
        (added, text, whole),
        (
            Some(Some(serde_json::json!(["1"]))),
            Some(format!(
                "{EMPTY_PLAN}| 1 | survey the river | 2 |  | Not started |  |\n"
            )),
            Some(kernel::Share::WHOLE)
        ),
        "the first line is a top-level row, and it holds the whole plan"
    );
}

#[test]
fn a_run_that_is_not_the_mayor_cannot_add_under_the_root() {
    let shared = empty_desk("lab/room1");
    let refused = ClaimTool::new(Arc::clone(&shared))
        .unwrap()
        .invoke(&call(
            serde_json::json!({ "action": "add", "parts": ["survey the river"] }),
        ))
        .err();
    assert_eq!(
        (refused, shared.lock().unwrap().roadmap().map(str::to_owned)),
        (
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "add a plan row",
                    "the plan's root belongs to the building's Mayor, and this run is lab/room1",
                )
                .with_recovery(
                    "claim a row and split it to add work under it, or signal hall/mayor to add \
                     a top-level row"
                )
            ),
            None
        )
    );
}

#[test]
fn split_writes_dependency_and_reports_each_unwritten_child() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({"action":"claim", "node":"1"})))
        .unwrap();
    tool.invoke(&call(serde_json::json!({
        "action":"split", "node":"1",
        "parts":[{"item":"wait for the glaze", "needs":["2"]}, "prepare the shelf"]
    })))
    .unwrap();
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1.1 | wait for the glaze | 1 | 2 | Not started |  |"));
    let list = tool
        .invoke(&call(serde_json::json!({"action":"list"})))
        .unwrap();
    assert_eq!(
        list.result.as_map()["pending"],
        serde_json::json!(["1", "1.1", "1.2"])
    );
    assert_eq!(
        list.result.as_map()["ready"],
        serde_json::json!([{"node":"1.2","item":"prepare the shelf"}])
    );
    let finished = set_roadmap_status(
        &text,
        &node("2"),
        RoadmapStatus::Done,
        Some(&Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap()),
    )
    .unwrap();
    let after = ClaimDesk::new(
        "potter@lab.2".to_owned(),
        Address::parse("lab/room1").unwrap(),
        finished,
        Booking::new(|_| Ok(())),
    );
    assert!(after.tree().unwrap().ready().contains(&node("1.1")));
}

#[test]
fn malformed_missing_or_circular_split_dependencies_leave_the_plan_and_hold_unchanged() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({"action":"claim", "node":"1"})))
        .unwrap();
    let before = shared.lock().unwrap().roadmap().unwrap().to_owned();
    for invalid in [
        serde_json::json!(["99"]),
        serde_json::json!(["1"]),
        serde_json::json!(["2", "2"]),
        serde_json::json!([""]),
        serde_json::json!(["1.1"]),
        serde_json::json!("2"),
    ] {
        let error = tool
            .invoke(&call(serde_json::json!({
                "action":"split", "node":"1", "parts":[{"item":"wait", "needs":invalid}]
            })))
            .err()
            .unwrap();
        assert_eq!(error.code(), &AxCode::InvalidArgs);
        let checked = shared.lock().unwrap();
        assert_eq!(checked.holding(), Some(&node("1")));
        assert_eq!(checked.roadmap(), Some(before.as_str()));
    }
    assert_eq!(shared.lock().unwrap().take_effects().len(), 1);
}

#[test]
fn mayor_add_persists_needs_and_refuses_a_missing_row_in_the_whole_batch() {
    let shared = Arc::new(Mutex::new(ClaimDesk::new(
        "mayor@hall.1".to_owned(),
        Address::parse("hall/mayor").unwrap(),
        PLAN.to_owned(),
        Booking::new(|_| Ok(())),
    )));
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let missing = tool.invoke(&call(serde_json::json!({
        "action":"add", "parts":["would be first", {"item":"also", "needs":["99"]}]
    })));
    assert!(missing.is_err());
    assert!(shared.lock().unwrap().roadmap().is_none());
    tool.invoke(&call(serde_json::json!({
        "action":"add", "parts":[{"item":"ship", "needs":["1","2"]}]
    })))
    .unwrap();
    let mutated = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(mutated.contains("| 4 | ship | 1 | 1, 2 | Not started |  |"));
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].apply(PLAN).unwrap(), mutated);
}

#[test]
fn replay_refuses_a_dependency_that_now_waits_for_the_split_branch() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({"action":"claim", "node":"1"})))
        .unwrap();
    tool.invoke(&call(serde_json::json!({
        "action":"split", "node":"1", "parts":[{"item":"wait for glaze", "needs":["3"]}]
    })))
    .unwrap();
    let effects = shared.lock().unwrap().take_effects();
    let latest = PLAN.replace(
        "| 2 | glaze tests | 1 |  | In progress |  |",
        "| 2 | glaze tests | 1 | 1 | In progress |  |",
    );
    let claimed = effects[0].apply(&latest).unwrap();
    let refusal = effects[1].apply(&claimed).unwrap_err();
    assert_eq!(refusal.code(), &AxCode::InvalidArgs);
    assert!(refusal.subject().contains("wait for 3"));
}
