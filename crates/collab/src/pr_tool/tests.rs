// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn request(branch: &str, implementer: &str) -> OpenRequest {
    OpenRequest {
        node: NodeId::parse(branch).unwrap(),
        implementer: implementer.to_owned(),
        branch: branch.to_owned(),
        commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
    }
}

fn tool(who: &str, branch: Option<&str>, open: Vec<OpenRequest>) -> (PrTool, Rc<RefCell<PrDesk>>) {
    let desk = Rc::new(RefCell::new(PrDesk::new(
        who.to_owned(),
        Address::parse("lab/room1").unwrap(),
        branch.map(str::to_owned),
        branch.and_then(|b| NodeId::parse(b).ok()),
        open,
    )));
    let tool = PrTool::new(Address::parse("lab/room1").unwrap(), Rc::clone(&desk)).unwrap();
    (tool, desk)
}

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("pr").unwrap(),
        args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
    }
}

#[test]
fn the_resident_who_wrote_it_cannot_be_the_one_who_checks_it() {
    let (mut tool, desk) = tool(
        "lab/room1",
        Some("tree-a"),
        vec![request("tree-a", "lab/room1")],
    );
    let refusal = tool
        .invoke(&call(serde_json::json!({
            "action": "check",
            "branch": "tree-a",
            "passed": true,
        })))
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::GateDenied);
    assert!(refusal.recovery().contains("another resident"));
    assert!(
        desk.borrow_mut().take_effects().is_empty(),
        "a refused check moves nothing"
    );
}

#[test]
fn a_check_that_passes_merges_and_one_that_fails_records_why() {
    let (mut checker, desk) = tool("lab/tests", None, vec![request("tree-a", "lab/room1")]);
    let outcome = checker
        .invoke(&call(serde_json::json!({
            "action": "check",
            "branch": "tree-a",
            "passed": true,
        })))
        .unwrap();
    assert_eq!(
        outcome
            .result
            .as_map()
            .get("merged")
            .and_then(Value::as_bool),
        Some(true)
    );
    let effects = desk.borrow_mut().take_effects();
    assert!(matches!(effects[0], PrEffect::Merged { .. }));

    let (mut second, desk) = tool("lab/tests", None, vec![request("tree-b", "lab/room1")]);
    second
        .invoke(&call(serde_json::json!({
            "action": "check",
            "branch": "tree-b",
            "passed": false,
            "why": "the tests do not run",
        })))
        .unwrap();
    let effects = desk.borrow_mut().take_effects();
    match &effects[0] {
        PrEffect::Rejected { why, .. } => assert_eq!(why, "the tests do not run"),
        other => panic!("a failed check rejects, not {other:?}"),
    }
}

#[test]
fn a_run_without_a_tree_has_nothing_to_offer_and_says_so() {
    let (mut tool, _desk) = tool("lab/room1", None, Vec::new());
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "open" })))
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::ToolUnavailable);
    assert!(refusal.recovery().contains("review"));
}

#[test]
fn one_branch_carries_one_request() {
    let (mut tool, _desk) = tool(
        "lab/room1",
        Some("tree-a"),
        vec![request("tree-a", "lab/room1")],
    );
    assert!(
        tool.invoke(&call(serde_json::json!({ "action": "open" })))
            .is_err()
    );
}

#[test]
fn a_request_reads_back_as_the_request_that_was_made() {
    let original = request("tree-a", "lab/room1");
    let read = OpenRequest::from_payload(&original.payload().unwrap()).unwrap();
    assert_eq!(read, original);
}

#[test]
fn listing_says_which_ones_are_yours() {
    let (mut tool, _desk) = tool(
        "lab/room1",
        Some("tree-a"),
        vec![
            request("tree-a", "lab/room1"),
            request("tree-b", "lab/room2"),
        ],
    );
    let outcome = tool
        .invoke(&call(serde_json::json!({ "action": "list" })))
        .unwrap();
    let rows = outcome
        .result
        .as_map()
        .get("requests")
        .and_then(Value::as_array)
        .unwrap()
        .clone();
    assert_eq!(rows.len(), 2);
    let mine: Vec<bool> = rows
        .iter()
        .filter_map(|row| row.get("yours").and_then(Value::as_bool))
        .collect();
    assert_eq!(mine, vec![true, false]);
}
