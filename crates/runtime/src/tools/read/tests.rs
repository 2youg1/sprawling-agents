// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::catalog::CatalogEntry;

fn tool(root: &Path) -> (ReadTool, Rc<RefCell<Catalog>>) {
    let catalog = Rc::new(RefCell::new(Catalog::new()));
    let tool = ReadTool::new(root, Rc::clone(&catalog)).unwrap();
    (tool, catalog)
}

fn call(path: &str) -> ToolCall {
    let mut args = Map::new();
    args.insert("path".to_owned(), Value::String(path.to_owned()));
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("read").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn interval(path: &str, offset: u64, limit: Option<u64>) -> ToolCall {
    let mut args = Map::new();
    args.insert("path".to_owned(), Value::String(path.to_owned()));
    args.insert("offset".to_owned(), Value::Number(offset.into()));
    if let Some(limit) = limit {
        args.insert("limit".to_owned(), Value::Number(limit.into()));
    }
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("read").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

#[test]
fn a_file_in_the_city_comes_back_with_its_own_length() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(dir.path().join("lab").join("Memo.md"), "one decision\n").unwrap();
    let (mut tool, _catalog) = tool(dir.path());

    let outcome = tool.invoke(&call("lab/Memo.md")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["text"], "one decision\n");
    assert_eq!(map["bytes"], 13);
}

/// The rule that keeps a run from reading its own governance is the
/// same predicate the write side uses, so there is one answer to
/// "what is reserved" rather than two.
#[test]
fn a_model_chosen_path_cannot_reach_a_reserved_subtree() {
    let dir = tempfile::tempdir().unwrap();
    let (mut tool, _catalog) = tool(dir.path());
    for asked in [
        ".sprawling/ledger/0001.jsonl",
        "lab/.sprawling/BUILDING.md",
        ".sprawling/CONFIG.toml",
    ] {
        let err = tool.invoke(&call(asked)).unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied, "{asked} was allowed");
        assert!(
            !err.recovery().is_empty(),
            "{asked} refused with no way out"
        );
    }
}

/// Admission happened when a person wrote the reading room, so the
/// skill it names opens even though it lives where no model-chosen
/// path may go.
#[test]
fn the_reading_room_hands_over_what_a_path_could_not_reach() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = dir.path().join(".sprawling").join("library");
    std::fs::create_dir_all(&shelf).unwrap();
    std::fs::write(shelf.join("review.md"), "check the diff first\n").unwrap();
    let (mut tool, catalog) = tool(dir.path());
    catalog
        .borrow_mut()
        .admit_skill(CatalogEntry {
            name: "review".to_owned(),
            disclosure: "how this building reviews".to_owned(),
            expansion: ".sprawling/library/review.md".to_owned(),
            hash: None,
        })
        .unwrap();

    assert!(
        tool.invoke(&call(".sprawling/library/review.md")).is_err(),
        "the path is still closed"
    );
    let outcome = tool.invoke(&call("review")).unwrap();
    assert_eq!(outcome.result.as_map()["text"], "check the diff first\n");
}

/// The catalog's own second level: the prompt carries one line per
/// entry, and this is the tool that fetches what the line stood for.
#[test]
fn an_entry_the_catalog_holds_is_handed_over_not_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (mut tool, catalog) = tool(dir.path());
    catalog.borrow_mut().set_mode(crate::mode::Mode::Experiment);

    let mode = tool.invoke(&call("mode:experiment")).unwrap();
    let said = mode.result.as_map()["text"].as_str().unwrap_or_default();
    assert!(said.contains("Memo.md"), "the mode's discipline: {said}");

    let dev = tool.invoke(&call("dev")).unwrap();
    let said = dev.result.as_map()["text"].as_str().unwrap_or_default();
    assert!(said.contains("-SPEC.md"), "the developer entry: {said}");
    assert!(said.contains("wait for the person to grant it"));
}

#[test]
fn a_missing_file_is_the_callers_mistake_not_the_disks() {
    let dir = tempfile::tempdir().unwrap();
    let (mut tool, _catalog) = tool(dir.path());
    let err = tool.invoke(&call("lab/nowhere.md")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

/// A file longer than the cap comes back one cap's worth at a time, and
/// the answer says what it is part of: the total, and the offset the
/// next call continues from. Without those two numbers a reader has to
/// guess whether it holds the whole thing.
#[test]
fn a_long_file_answers_with_the_cap_a_total_and_the_next_offset() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let body: String = (0..700).map(|n| format!("line {n}\n")).collect();
    std::fs::write(dir.path().join("lab").join("Long.md"), &body).unwrap();
    let (mut tool, _catalog) = tool(dir.path());

    let first = tool.invoke(&call("lab/Long.md")).unwrap();
    let map = first.result.as_map();
    let text = map["text"].as_str().unwrap();
    assert_eq!(text.lines().count(), 512, "the cap is what came back");
    assert!(text.starts_with("line 0\n"));
    assert_eq!(map["total_lines"], 700);
    assert_eq!(map["next_offset"], 512);

    let rest = tool.invoke(&interval("lab/Long.md", 512, None)).unwrap();
    let map = rest.result.as_map();
    let text = map["text"].as_str().unwrap();
    assert_eq!(text.lines().count(), 188);
    assert!(text.starts_with("line 512\n"));
    assert!(
        !map.contains_key("next_offset"),
        "the end of a file has nothing after it"
    );
}

/// A caller that asks for more than the cap is served the cap rather
/// than refused: a refusal would cost it a turn to learn a number the
/// answer can carry.
#[test]
fn a_limit_over_the_cap_is_clamped_and_a_short_file_stays_whole() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let body: String = (0..600).map(|n| format!("line {n}\n")).collect();
    std::fs::write(dir.path().join("lab").join("Long.md"), &body).unwrap();
    std::fs::write(dir.path().join("lab").join("Memo.md"), "one decision\n").unwrap();
    let (mut tool, _catalog) = tool(dir.path());

    let capped = tool
        .invoke(&interval("lab/Long.md", 0, Some(9_000)))
        .unwrap();
    let map = capped.result.as_map();
    assert_eq!(map["text"].as_str().unwrap().lines().count(), 512);
    assert_eq!(map["total_lines"], 600);

    let whole = tool.invoke(&call("lab/Memo.md")).unwrap();
    let map = whole.result.as_map();
    assert_eq!(map["text"], "one decision\n");
    assert_eq!(map["bytes"], 13);
    assert!(!map.contains_key("next_offset"));
}

/// Past the end is not an error and not a loop: the total says why the
/// answer is empty, and no `next_offset` invites another call.
#[test]
fn an_offset_past_the_end_answers_empty_with_the_total() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(dir.path().join("lab").join("Memo.md"), "one decision\n").unwrap();
    let (mut tool, _catalog) = tool(dir.path());

    let outcome = tool.invoke(&interval("lab/Memo.md", 40, None)).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["text"], "");
    assert_eq!(map["total_lines"], 1);
    assert!(!map.contains_key("next_offset"));
}

#[test]
fn an_interval_that_cannot_be_counted_is_refused_rather_than_guessed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(dir.path().join("lab").join("Memo.md"), "one decision\n").unwrap();
    let (mut tool, _catalog) = tool(dir.path());

    for bad in [
        Value::String("12".to_owned()),
        Value::Number((-3).into()),
        Value::Number(0.into()),
    ] {
        let mut args = Map::new();
        args.insert("path".to_owned(), Value::String("lab/Memo.md".to_owned()));
        args.insert("limit".to_owned(), bad.clone());
        let err = tool
            .invoke(&ToolCall {
                id: "call-1".to_owned(),
                name: ToolName::parse("read").unwrap(),
                args: Payload::new(args).unwrap(),
            })
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs, "limit {bad} was allowed");
    }
}

#[cfg(feature = "conformance")]
#[test]
fn the_tool_refuses_another_tools_call_and_still_answers() {
    let dir = tempfile::tempdir().unwrap();
    let (mut tool, _catalog) = tool(dir.path());
    kernel::tool_conformance::assert_tool_conformance(&mut tool);
}
