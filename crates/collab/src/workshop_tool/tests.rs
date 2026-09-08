// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::fanin::Claim;
use kernel::{B3Hash, Depth, Locator};

fn tool() -> (
    WorkshopTool,
    std::sync::Arc<std::sync::Mutex<WorkshopDesk>>,
    std::sync::Arc<std::sync::Mutex<DelegateDesk>>,
) {
    let desk = std::sync::Arc::new(std::sync::Mutex::new(WorkshopDesk::new(
        "lab/room1".to_owned(),
        FanIn::new(),
    )));
    let delegates = std::sync::Arc::new(std::sync::Mutex::new(DelegateDesk::new(
        Depth::Root,
        Address::parse("lab").unwrap(),
    )));
    let tool = WorkshopTool::new(
        std::sync::Arc::clone(&desk),
        std::sync::Arc::clone(&delegates),
    )
    .unwrap();
    (tool, desk, delegates)
}

fn lay_out(nodes: Value) -> ToolCall {
    let mut args = Map::new();
    args.insert("op".to_owned(), Value::String("lay_out".to_owned()));
    args.insert("nodes".to_owned(), nodes);
    ToolCall {
        id: "c1".to_owned(),
        name: ToolName::parse("workshop").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn node(room: &str, depends_on: &[&str]) -> Value {
    serde_json::json!({
        "room": room,
        "goal": format!("build {room}"),
        "done_check": "the tests pass",
        "stop": "when the tests pass, or after three attempts",
        "depends_on": depends_on,
    })
}

#[test]
fn a_graph_is_handed_down_in_dependency_order_and_each_node_carries_its_contract() {
    let (mut tool, _desk, delegates) = tool();
    let outcome = tool
        .invoke(&lay_out(serde_json::json!([
            node("lab/writer", &["lab/reader"]),
            node("lab/reader", &[]),
        ])))
        .unwrap();
    let schedule: Vec<&str> = outcome.result.as_map()["schedule"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(schedule, ["lab/reader", "lab/writer"]);

    let handed = delegates.lock().unwrap().take();
    assert_eq!(handed.len(), 2);
    assert_eq!(handed[0].room.as_str(), "lab/reader");
    assert!(
        handed[0].task.contains("## Done check"),
        "the node's job file is its contract, not a summary of it: {}",
        handed[0].task
    );
}

/// The graph refuses itself before anything is handed down, so a
/// cycle never becomes half a workshop.
#[test]
fn a_cycle_is_refused_and_nothing_is_handed_down() {
    let (mut tool, _desk, delegates) = tool();
    let err = tool
        .invoke(&lay_out(serde_json::json!([
            node("lab/a", &["lab/b"]),
            node("lab/b", &["lab/a"]),
        ])))
        .unwrap_err();
    assert!(err.recovery().contains("cycle"));
    assert!(delegates.lock().unwrap().take().is_empty());
}

#[test]
fn one_run_lays_out_one_graph() {
    let (mut tool, _desk, _delegates) = tool();
    tool.invoke(&lay_out(serde_json::json!([node("lab/a", &[])])))
        .unwrap();
    let err = tool
        .invoke(&lay_out(serde_json::json!([node("lab/b", &[])])))
        .unwrap_err();
    assert!(err.recovery().contains("one graph per session"));
}

/// The join's fence, reached through the tool: an answer that could
/// have been written without opening anything is refused.
#[test]
fn the_join_will_not_take_a_verdict_from_somebody_who_read_nothing() {
    let (mut tool, desk, _delegates) = tool();
    let content = b"what the node produced";
    let digest = B3Hash::digest(content);
    let artifact = Claim::new(
        NodeId::parse("lab/reader").unwrap(),
        Locator::parse(&format!("cas:b3-{digest}")).unwrap(),
        digest,
        "lab/reader".to_owned(),
    )
    .verified(true, "city")
    .unwrap();
    desk.lock().unwrap().accept(artifact);

    let mut ask = Map::new();
    ask.insert("op".to_owned(), Value::String("question".to_owned()));
    let asked = tool
        .invoke(&ToolCall {
            id: "c2".to_owned(),
            name: ToolName::parse("workshop").unwrap(),
            args: Payload::new(ask).unwrap(),
        })
        .unwrap();
    assert!(
        asked.result.as_map()["question"]
            .as_str()
            .unwrap()
            .contains("digest")
    );

    let judge = |answer: &str| {
        let mut args = Map::new();
        args.insert("op".to_owned(), Value::String("judge".to_owned()));
        args.insert("answer".to_owned(), Value::String(answer.to_owned()));
        ToolCall {
            id: "c3".to_owned(),
            name: ToolName::parse("workshop").unwrap(),
            args: Payload::new(args).unwrap(),
        }
    };
    assert!(tool.invoke(&judge("looks right to me")).is_err());
    let witness: String = digest.to_string().chars().take(8).collect();
    let joined = tool.invoke(&judge(&witness)).unwrap();
    assert_eq!(
        joined.result.as_map()["joined"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn an_unknown_verb_is_refused_rather_than_rounded_to_the_harmless_one() {
    let (mut tool, _desk, _delegates) = tool();
    let mut args = Map::new();
    args.insert("op".to_owned(), Value::String("close".to_owned()));
    let err = tool
        .invoke(&ToolCall {
            id: "c4".to_owned(),
            name: ToolName::parse("workshop").unwrap(),
            args: Payload::new(args).unwrap(),
        })
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}
