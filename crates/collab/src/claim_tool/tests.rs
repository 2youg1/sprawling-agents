// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{Tool, ToolCall, ToolName};

use super::tool::ACTIONS;
use super::*;
// The desk produces the effects; whether one still holds is
// `claim_effect`'s question, asked here against a real desk run.
use crate::claim_effect::{evidence_of, still_true};

const PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | wire the kiln | 1 |  | Not started |  |
| 2 | glaze tests | 1 |  | In progress |  |
| 3 | fire the batch | 1 | 2 | Not started |  |
";

fn desk() -> Arc<Mutex<ClaimDesk>> {
    Arc::new(Mutex::new(ClaimDesk::new(
        "potter@lab.1".to_owned(),
        Address::parse("lab/room1").unwrap(),
        PLAN.to_owned(),
    )))
}

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("plan").unwrap(),
        args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
    }
}

fn locator() -> String {
    format!("cas:b3-{}", "ab".repeat(32))
}

fn node(raw: &str) -> NodeId {
    NodeId::parse(raw).unwrap()
}

#[test]
fn claiming_a_ready_node_marks_it_and_queues_one_effect() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    assert_eq!(
        outcome.result.as_map().get("item").and_then(Value::as_str),
        Some("wire the kiln")
    );
    let mut borrowed = shared.lock().unwrap();
    let text = borrowed.roadmap().expect("the plan changed").to_owned();
    assert!(text.contains("| 1 | wire the kiln | 1 |  | In progress |  |"));
    let effects = borrowed.take_effects();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].id(), &node("1"));
    assert_eq!(effects[0].kind(), kernel::EventKind::RoadmapClaimed);
    assert!(
        borrowed.take_effects().is_empty(),
        "an effect read twice would be a claim recorded twice"
    );
}

#[test]
fn a_node_somebody_else_is_working_on_is_refused_with_a_ready_one() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "claim", "node": "2" })))
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::GoalConflict);
    assert!(refusal.subject().contains("In progress"));
    assert!(
        refusal.recovery().contains("claim 1"),
        "the third part names a node the caller may actually take: {}",
        refusal.recovery()
    );
    assert!(
        shared.lock().unwrap().roadmap().is_none(),
        "a refusal writes nothing"
    );
}

/// The ready set is what `list` offers: a node whose dependency is
/// not done is not work anybody can take.
#[test]
fn listing_offers_what_is_ready_rather_than_what_is_merely_unclaimed() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({ "action": "list" })))
        .unwrap();
    let map = outcome.result.as_map();
    let ready = map.get("ready").and_then(Value::as_array).unwrap();
    assert_eq!(ready.len(), 1, "3 waits for 2");
    assert_eq!(ready[0].get("node").and_then(Value::as_str), Some("1"));
    assert_eq!(
        map.get("in_progress")
            .and_then(Value::as_array)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(map.get("nodes_total").and_then(Value::as_u64), Some(3));
    assert!(
        shared.lock().unwrap().roadmap().is_none(),
        "reading the plan does not modify it"
    );
}

#[test]
fn a_run_holds_one_node_at_a_time() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "claim", "node": "3" })))
        .unwrap_err();
    assert!(refusal.subject().contains("holds 1"));
    assert!(refusal.recovery().contains("one node"));
}

#[test]
fn finishing_requires_evidence_that_parses() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let missing = tool
        .invoke(&call(
            serde_json::json!({ "action": "finish", "node": "1" }),
        ))
        .unwrap_err();
    assert_eq!(missing.code(), &AxCode::EvidenceMissing);
    let unparsable = tool
        .invoke(&call(serde_json::json!({
            "action": "finish", "node": "1", "evidence": "trust me"
        })))
        .unwrap_err();
    assert_eq!(unparsable.code(), &AxCode::LocatorInvalid);
    let done = tool
        .invoke(&call(serde_json::json!({
            "action": "finish", "node": "1", "evidence": locator()
        })))
        .unwrap();
    assert!(done.result.as_map().contains_key("evidence"));
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert_eq!(
        evidence_of(&text, &node("1")).map(|l| l.to_string()),
        Some(locator()),
        "what was written is what a reader retrieves"
    );
}

/// The plan gate, from the tool's side: a run may only put down the
/// node it took, and it may not close one it never claimed.
#[test]
fn a_run_can_only_put_down_what_it_took() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let never = tool
        .invoke(&call(serde_json::json!({
            "action": "finish", "node": "2", "evidence": locator()
        })))
        .unwrap_err();
    assert!(never.subject().contains("holds nothing"));
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let other = tool
        .invoke(&call(serde_json::json!({
            "action": "finish", "node": "2", "evidence": locator()
        })))
        .unwrap_err();
    assert!(other.subject().contains("holds 1, not 2"));
}

#[test]
fn blocking_paints_the_node_red_and_says_why() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "action": "block", "node": "1", "reason": "the kiln has no power"
        })))
        .unwrap();
    assert_eq!(
        outcome.result.as_map().get("red").and_then(Value::as_bool),
        Some(true)
    );
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1 | wire the kiln | 1 |  | Blocked |  |"));
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects[1].kind(), kernel::EventKind::RoadmapBlocked);
    let payload = effects[1].payload("potter@lab.1").unwrap();
    assert_eq!(
        payload.as_map().get("verb").and_then(Value::as_str),
        Some("blocked")
    );
    assert!(
        payload
            .as_map()
            .get("line")
            .and_then(Value::as_str)
            .is_some_and(|line| line.contains("no power")),
        "the reason travels with the record, not only with the row"
    );
}

#[test]
fn releasing_puts_the_node_back_where_another_run_can_take_it() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "action": "release", "node": "1", "reason": "not my trade"
        })))
        .unwrap();
    assert_eq!(
        outcome.result.as_map().get("red").and_then(Value::as_bool),
        Some(false),
        "handing back is not red"
    );
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1 | wire the kiln | 1 |  | Not started |  |"));
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects[1].kind(), kernel::EventKind::RoadmapReleased);
}

#[test]
fn putting_a_node_down_without_a_reason_is_refused() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "block", "node": "1" })))
        .unwrap_err();
    assert!(refusal.recovery().contains("one line"));
}

#[test]
fn splitting_grows_the_plan_and_the_run_stops_holding_the_branch() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
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

/// A run that ends holding a node leaves red behind, and the reason
/// says what happened rather than inventing one.
#[test]
fn a_run_that_freezes_still_holding_a_node_leaves_it_red() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    shared.lock().unwrap().abandon().unwrap();
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1 | wire the kiln | 1 |  | Blocked |  |"));
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects[1].kind(), kernel::EventKind::RoadmapBlocked);
    assert!(
        shared.lock().unwrap().abandon().is_ok(),
        "a run that put its node down properly abandons nothing"
    );
}

#[test]
fn an_effect_whose_node_moved_underneath_it_is_no_longer_true() {
    let shared = desk();
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let effects = shared.lock().unwrap().take_effects();
    assert!(
        still_true(PLAN, &effects[0]),
        "against the file it was decided on, the effect holds"
    );
    let moved = set_roadmap_status(PLAN, &node("1"), RoadmapStatus::InProgress, None).unwrap();
    assert!(
        !still_true(&moved, &effects[0]),
        "somebody else took the node first; the claim does not take"
    );
}

#[test]
fn a_plan_that_does_not_parse_refuses_with_the_repair() {
    let shared = Arc::new(Mutex::new(ClaimDesk::new(
        "potter@lab.1".to_owned(),
        Address::parse("lab/room1").unwrap(),
        "no table here".to_owned(),
    )));
    let mut tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "list" })))
        .unwrap_err();
    assert!(refusal.recovery().contains("six-column"));
}

#[test]
fn an_unknown_action_is_answered_with_the_six_that_exist() {
    let mut tool = ClaimTool::new(desk()).unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "delete" })))
        .unwrap_err();
    assert_eq!(refusal.recovery(), ACTIONS);
}

#[test]
fn the_tool_refuses_a_call_bearing_another_tools_name() {
    let mut tool = ClaimTool::new(desk()).unwrap();
    let refusal = tool
        .invoke(&ToolCall {
            id: "tu_x".to_owned(),
            name: ToolName::parse("signal").unwrap(),
            args: Payload::empty(),
        })
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::InvalidArgs);
    assert_eq!(
        tool.meta().name.as_str(),
        "plan",
        "a refusal must not poison the tool"
    );
}

/// Six actions cost no more catalog bytes than four did. The catalog is what every turn pays for, so a
/// verb that grows it is a verb charged to every run in the city
/// whether or not it is ever called.
#[test]
fn six_actions_cost_no_more_catalog_bytes_than_four_did() {
    let tool = ClaimTool::new(desk()).unwrap();
    let meta = tool.meta();
    let bytes = meta.disclosure.len()
        + serde_json::to_string(meta.params.as_map())
            .expect("the schema serialises")
            .len();
    // The four-action tool measured 548 B on this same reading. Two
    // more verbs and two more arguments fit under it because the
    // locator grammar left the schema for the refusal that needs it:
    // a description repeating what a refusal already says is paid
    // for every turn and read once.
    assert!(
        bytes <= 548,
        "the plan entry costs {bytes} B, and four actions cost 548"
    );
}
