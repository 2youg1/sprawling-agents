// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn call(args: &[(&str, Value)]) -> ToolCall {
    let mut map = Map::new();
    for (key, value) in args {
        map.insert((*key).to_owned(), value.clone());
    }
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("search").unwrap(),
        args: Payload::new(map).unwrap(),
    }
}

fn text(what: &str) -> ToolCall {
    call(&[("text", Value::String(what.to_owned()))])
}

fn city() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let lab = dir.path().join("lab").join("room1");
    std::fs::create_dir_all(&lab).unwrap();
    std::fs::write(
        lab.join("Memo.md"),
        "one decision\nthe ledger is the history\nnothing else\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("lab").join("Notes.md"),
        "before\nthe ledger again\nafter\n",
    )
    .unwrap();
    let reserved = dir.path().join(".sprawling");
    std::fs::create_dir_all(&reserved).unwrap();
    std::fs::write(reserved.join("CONFIG.toml"), "the ledger governs\n").unwrap();
    dir
}

/// A hit carries where it is and what surrounds it, and the line number
/// is the one `read`'s `offset` uses, so continuing is passing a number
/// along rather than converting one.
#[test]
fn a_substring_comes_back_with_its_place_and_its_context() {
    let dir = city();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let outcome = tool
        .invoke(&call(&[
            ("text", Value::String("ledger is".to_owned())),
            ("context", Value::Number(1.into())),
        ]))
        .unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["count"], 1);
    assert_eq!(map["truncated"], Value::Bool(false));
    let hits = map["matches"].as_array().unwrap();
    let hit = hits[0].as_object().unwrap();
    assert_eq!(hit["path"], "lab/room1/Memo.md");
    assert_eq!(hit["line"], 1);
    assert_eq!(
        hit["text"],
        "one decision\nthe ledger is the history\nnothing else"
    );
}

#[test]
fn every_file_under_the_prefix_is_looked_at_and_nothing_above_it() {
    let dir = city();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let whole = tool.invoke(&text("the ledger")).unwrap();
    assert_eq!(whole.result.as_map()["count"], 2, "two files, one each");

    let one_room = tool
        .invoke(&call(&[
            ("text", Value::String("the ledger".to_owned())),
            ("path", Value::String("lab/room1".to_owned())),
        ]))
        .unwrap();
    assert_eq!(one_room.result.as_map()["count"], 1);
}

/// The same predicate `read` uses, reached through the same function:
/// a prefix inside reserved space is refused, and the walk of an
/// ordinary prefix never descends into one.
#[test]
fn a_reserved_prefix_is_refused_and_a_reserved_file_is_never_matched() {
    let dir = city();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let err = tool
        .invoke(&call(&[
            ("text", Value::String("the ledger".to_owned())),
            ("path", Value::String(".sprawling".to_owned())),
        ]))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
    assert!(!err.recovery().is_empty());

    let whole = tool.invoke(&text("governs")).unwrap();
    assert_eq!(
        whole.result.as_map()["count"],
        0,
        "the reserved file was read anyway"
    );
}

#[test]
fn a_file_that_is_not_text_is_passed_over_rather_than_reported() {
    let dir = city();
    std::fs::write(dir.path().join("lab").join("blob.bin"), [0xff, 0xfe, 0x00]).unwrap();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    assert_eq!(outcome.result.as_map()["count"], 2);
}

#[test]
fn an_empty_predicate_is_refused_because_it_would_match_everything() {
    let dir = city();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let err = tool.invoke(&text("")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    let err = tool.invoke(&call(&[])).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

/// The cap exists so one call cannot spend a whole window, and the
/// answer says the cap was reached rather than looking complete.
#[test]
fn the_cap_stops_the_walk_and_the_answer_says_so() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let body: String = (0..200).map(|n| format!("the ledger {n}\n")).collect();
    std::fs::write(dir.path().join("lab").join("Many.md"), body).unwrap();
    let mut tool = SearchTool::new(dir.path()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["count"], 64);
    assert_eq!(map["truncated"], Value::Bool(true));
}

#[cfg(feature = "conformance")]
#[test]
fn the_tool_refuses_another_tools_call_and_still_answers() {
    let dir = tempfile::tempdir().unwrap();
    let mut tool = SearchTool::new(dir.path()).unwrap();
    kernel::tool_conformance::assert_tool_conformance(&mut tool);
}
