// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

fn snapshot() -> StatusSnapshot {
    StatusSnapshot {
        who: "alice".to_owned(),
        addr: Address::parse("work").unwrap(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        ctx_limit: Tokens::new(8000),
        trust: "trusted".to_owned(),
        write_domain: "work".to_owned(),
        locks: vec!["work/a.txt".to_owned()],
        worktree_path: "/city/work".to_owned(),
        worktree_disk: ByteLen::new(4096),
        signals_pending: 2,
        provider_mode: ProviderMode::Normal,
        neighbours: 3,
    }
}

fn call() -> ToolCall {
    ToolCall {
        id: "s1".to_owned(),
        name: ToolName::parse("status").unwrap(),
        args: Payload::new(Map::new()).unwrap(),
    }
}

#[test]
fn the_thirteen_fields_report_in_the_frozen_order() {
    let tool = StatusTool::new(snapshot()).unwrap();
    let outcome = tool.invoke(&call()).unwrap();
    let value = serde_json::to_value(&outcome.result).unwrap();
    let text = value["text"].as_str().unwrap();
    let order = [
        "who",
        "addr",
        "mode",
        "ctx",
        "trust",
        "write_domain",
        "worktree",
        "signals_pending",
        "children",
        "now",
        "provider_mode",
        "neighbours",
        "backlog",
    ];
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        13,
        "the frozen list is thirteen fields: {text}"
    );
    for (line, field) in lines.iter().zip(order) {
        assert!(
            line.starts_with(&format!("{field}:")),
            "expected {field}, got {line}"
        );
    }
}

/// `children` was a hardcoded empty list, so a run that had just
/// handed work down and then asked about its own situation was told
/// it had handed nothing down.
#[test]
fn the_children_line_says_where_the_work_went_and_which_kind_of_delegate() {
    let handed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = std::sync::Arc::clone(&handed);
    let tool =
        StatusTool::watching(snapshot(), Box::new(move || seen.lock().unwrap().clone())).unwrap();

    let before = serde_json::to_value(&tool.invoke(&call()).unwrap().result).unwrap();
    assert!(before["text"].as_str().unwrap().contains("children: none"));

    handed.lock().unwrap().push(ChildStatus {
        room: Address::parse("work/helper").unwrap(),
        kind: DelegateKind::Ephemeral,
    });
    let after = serde_json::to_value(&tool.invoke(&call()).unwrap().result).unwrap();
    assert!(
        after["text"]
            .as_str()
            .unwrap()
            .contains("children: work/helper (ephemeral)"),
        "the desk is asked at call time, not frozen with the tool: {after}"
    );
}

#[test]
fn the_tool_reports_what_it_was_given_and_never_samples() {
    let tool = StatusTool::new(snapshot()).unwrap();
    let first = tool.invoke(&call()).unwrap();
    let second = tool.invoke(&call()).unwrap();
    assert_eq!(
        first.result, second.result,
        "two calls, one turn, one answer"
    );
}

#[test]
fn a_call_for_another_tool_is_refused() {
    let tool = StatusTool::new(snapshot()).unwrap();
    let mut wrong = call();
    wrong.name = ToolName::parse("edit").unwrap();
    assert_eq!(
        *match tool.invoke(&wrong) {
            Err(err) => err,
            Ok(_) => panic!("identity is fail-closed"),
        }
        .code(),
        AxCode::InvalidArgs
    );
}

/// The `now` line was frozen with the snapshot and never filled, so a
/// model that asked what time it was read `not stamped` in every city.
#[test]
fn the_now_line_reports_the_drivers_latest_reading_in_iso_utc() {
    let clock = ClockReading::default();
    let tool = StatusTool::new(snapshot()).unwrap().clocked(clock.clone());
    let now_line = |tool: &StatusTool| {
        let value = serde_json::to_value(&tool.invoke(&call()).unwrap().result).unwrap();
        value["text"]
            .as_str()
            .unwrap()
            .lines()
            .find(|line| line.starts_with("now:"))
            .unwrap()
            .to_owned()
    };
    assert_eq!(now_line(&tool), "now: not stamped");
    clock.keep(TimeMs::new(1_785_585_607_000));
    assert_eq!(now_line(&tool), "now: 2026-08-01T12:00:07Z");
}
