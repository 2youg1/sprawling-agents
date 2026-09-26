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

fn names(outcome: &ToolOutcome, field: &str) -> Vec<String> {
    outcome.result.as_map()[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

fn joined(room: &str) -> Artifact {
    let digest = B3Hash::digest(room.as_bytes());
    Claim::new(
        NodeId::parse(room).unwrap(),
        Locator::parse(&format!("cas:b3-{digest}")).unwrap(),
        digest,
        room.to_owned(),
    )
    .verified(true, "city")
    .unwrap()
}

/// A node whose dependency has not joined would read an output that
/// does not exist yet, so it waits; the schedule still names the whole
/// graph.
#[test]
fn only_the_nodes_whose_dependencies_have_joined_are_handed_down() {
    let (mut first, _desk, delegates) = tool();
    let graph = serde_json::json!([
        node("lab/writer", &["lab/reader"]),
        node("lab/reader", &[]),
    ]);
    let outcome = first.invoke(&lay_out(graph.clone())).unwrap();
    let handed = delegates.lock().unwrap().take();
    let rooms: Vec<&str> = handed.iter().map(|work| work.room.as_str()).collect();
    assert_eq!(rooms, ["lab/reader"]);
    assert_eq!(names(&outcome, "schedule"), ["lab/reader", "lab/writer"]);
    assert_eq!(names(&outcome, "handed"), ["lab/reader"]);
    assert_eq!(names(&outcome, "waiting"), ["lab/writer"]);
    assert!(
        handed[0].task.contains("## Done check"),
        "the node's job file is its contract, not a summary of it: {}",
        handed[0].task
    );

    // A later run of the room, once the reader has handed back, lays the
    // same graph out and hands down the writer alone.
    let (mut later, desk, delegates) = tool();
    desk.lock().unwrap().accept(joined("lab/reader"));
    let outcome = later.invoke(&lay_out(graph)).unwrap();
    assert_eq!(names(&outcome, "handed"), ["lab/writer"]);
    let rooms: Vec<String> = delegates
        .lock()
        .unwrap()
        .take()
        .iter()
        .map(|work| work.room.as_str().to_owned())
        .collect();
    assert_eq!(rooms, ["lab/writer"]);
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
