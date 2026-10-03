// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, ReadVerdict};

use super::*;

/// A bound under which every building is open, so a test about the walk
/// judges nothing else.
fn everywhere() -> ReadBound {
    std::sync::Arc::new(|_: &Address| ReadVerdict::Open)
}

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
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

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
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

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
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

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
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    assert_eq!(outcome.result.as_map()["count"], 2);
}

#[test]
fn an_empty_predicate_is_refused_because_it_would_match_everything() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

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
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["count"], 64);
    assert_eq!(map["truncated"], Value::Bool(true));
}

/// A search with no path walks what this run may read and nothing
/// else: a building the bound closes is not entered, and the bound is
/// asked once per building at the city root rather than once per file.
#[test]
fn a_walk_from_the_root_does_not_enter_a_building_the_bound_closes() {
    let dir = city();
    let vault = dir.path().join("vault").join("room1");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("notes.md"), "the ledger of the vault\n").unwrap();
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = std::sync::Arc::clone(&asked);
    let bound: ReadBound = std::sync::Arc::new(move |addr: &Address| {
        seen.lock().unwrap().push(addr.as_str().to_owned());
        if addr.as_str().starts_with("vault") {
            ReadVerdict::Confidential
        } else {
            ReadVerdict::Open
        }
    });
    let tool = SearchTool::new(dir.path(), bound).unwrap();

    let whole = tool.invoke(&text("the ledger")).unwrap();
    let map = whole.result.as_map();
    assert_eq!(map["count"], 2, "the two hits in lab, none in the vault");
    assert_eq!(
        map["unreadable"], 0,
        "a closed building is policy, not a failure"
    );
    let mut buildings = asked.lock().unwrap().clone();
    buildings.sort();
    assert_eq!(
        buildings,
        ["lab", "vault"],
        "once per building, never per file"
    );

    let err = tool
        .invoke(&call(&[
            ("text", Value::String("the ledger".to_owned())),
            ("path", Value::String("vault/room1".to_owned())),
        ]))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
}

/// A file too large to search is a place the walk did not look, so it
/// is counted and named with the reason, and the answer says how to get
/// at it instead.
#[test]
fn a_file_past_the_byte_cap_is_reported_with_its_reason() {
    let dir = city();
    let big = "the ledger\n".repeat(100_000);
    std::fs::write(dir.path().join("lab").join("Big.md"), big).unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["count"], 2, "the large file was not searched");
    assert_eq!(map["unreadable"], 1);
    let unread = map["unread"].as_array().unwrap();
    assert_eq!(unread[0]["path"], "lab/Big.md");
    assert!(
        unread[0]["why"].as_str().unwrap().contains("interval"),
        "{}",
        unread[0]["why"]
    );
}

/// One hit whose own line is wider than the byte budget is cut to the
/// budget and marked, instead of travelling whole: a single line of a
/// generated file would otherwise spend the window by itself.
#[test]
fn a_first_hit_wider_than_the_budget_is_cut_to_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let wide = format!("the ledger {}\n", "x".repeat(cap.saturating_mul(2)));
    std::fs::write(dir.path().join("lab").join("Wide.md"), wide).unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    let hit = map["matches"][0]["text"].as_str().unwrap();
    assert!(hit.len() <= cap, "{} bytes came back", hit.len());
    assert!(hit.starts_with("the ledger"));
    assert!(hit.ends_with(" bytes]"), "the cut is marked");
    assert_eq!(map["truncated"], Value::Bool(true));
}

#[cfg(feature = "conformance")]
#[test]
fn the_tool_refuses_another_tools_call_and_still_answers() {
    let dir = tempfile::tempdir().unwrap();
    let mut tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    kernel::tool::conformance::assert_tool_conformance(&mut tool);
}

/// A walk through an open building does not step through a link into a
/// confidential one: the link is judged where it lands, and a link to a
/// directory is not entered at all, because a walk through links can
/// come back to where it started.
#[test]
fn a_walk_does_not_follow_a_link_into_a_closed_building() {
    let dir = city();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(vault.join("secret.md"), "the ledger of the vault\n").unwrap();
    super::super::chosen_path::make_link(&dir.path().join("lab").join("to-vault"), &vault);
    super::super::chosen_path::make_link(&dir.path().join("lab").join("loop"), dir.path());
    let only_lab: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str().starts_with("vault") {
            ReadVerdict::Confidential
        } else {
            ReadVerdict::Open
        }
    });
    let tool = SearchTool::new(dir.path(), only_lab).unwrap();

    for asked in [None, Some("lab")] {
        let mut args = vec![("text", Value::String("ledger".to_owned()))];
        args.extend(asked.map(|path| ("path", Value::String(path.to_owned()))));
        let outcome = tool.invoke(&call(&args)).unwrap();
        let paths: Vec<&str> = outcome.result.as_map()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| hit["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            ["lab/Notes.md", "lab/room1/Memo.md"],
            "from {asked:?}"
        );
    }
}

/// A link whose real location does not resolve is not a closed
/// building: the disk failed to answer, so the walk counts it as a place
/// it could not look and names the storage failure, instead of passing
/// over it the way it passes over a link the read bound refuses.
#[test]
fn a_link_that_does_not_resolve_is_counted_as_unread() {
    let dir = city();
    let knot = dir.path().join("lab").join("knot");
    super::super::chosen_path::make_link(&knot, &knot);
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["unreadable"], 1, "{map:?}");
    let unread = map["unread"].as_array().unwrap();
    assert_eq!(unread[0]["path"], "lab/knot");
    assert!(
        unread[0]["why"]
            .as_str()
            .unwrap()
            .contains("E_STORAGE_FATAL"),
        "{}",
        unread[0]["why"]
    );
}

/// A building whose rules do not read is closed like a confidential
/// one, but the closing is a failure a person has to fix, so the walk
/// names it instead of answering as if it held nothing.
#[test]
fn a_building_whose_rules_do_not_read_is_named_among_the_unread() {
    let dir = city();
    std::fs::create_dir_all(dir.path().join("broken").join("room1")).unwrap();
    let bound: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str().starts_with("broken") {
            ReadVerdict::RulesUnreadable(
                AxError::failure(
                    kernel::AxCode::InvalidArgs,
                    "read the rules",
                    "broken/RULES.toml",
                )
                .with_recovery("fix the rules"),
            )
        } else {
            ReadVerdict::Open
        }
    });
    let tool = SearchTool::new(dir.path(), bound).unwrap();

    let whole = tool.invoke(&text("the ledger")).unwrap();
    let map = whole.result.as_map();
    assert_eq!(map["count"], 2, "the two hits in lab");
    assert_eq!(map["unreadable"], 1, "{:?}", map["unread"]);
    assert_eq!(map["unread"][0]["path"], "broken");
}

/// `unread` lists what was not looked at in the order the walk met it,
/// so a building whose rules fail sits between the buildings named
/// before and after it, not ahead of every file the walk read.
#[test]
fn the_unread_list_follows_the_walk_order() {
    let dir = city();
    for building in ["broken", "mill"] {
        std::fs::create_dir_all(dir.path().join(building).join("room1")).unwrap();
    }
    std::fs::write(
        dir.path().join("lab").join("Big.md"),
        "the ledger\n".repeat(100_000),
    )
    .unwrap();
    let bound: ReadBound = std::sync::Arc::new(|addr: &Address| {
        if addr.as_str() == "lab" {
            ReadVerdict::Open
        } else {
            ReadVerdict::RulesUnreadable(
                AxError::failure(kernel::AxCode::InvalidArgs, "read the rules", addr.as_str())
                    .with_recovery("fix the rules"),
            )
        }
    });
    let tool = SearchTool::new(dir.path(), bound).unwrap();

    let whole = tool.invoke(&text("the ledger")).unwrap();
    let paths: Vec<&Value> = whole.result.as_map()["unread"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| &entry["path"])
        .collect();
    assert_eq!(paths, ["broken", "lab/Big.md", "mill"]);
}

/// A name the city cannot spell as an address is named among the unread
/// with the parse's reason; Windows cannot make one, so the walk is asked.
#[test]
fn a_name_that_is_no_address_is_named_among_the_unread() {
    let named = match descent::admissible("lab", "a:b", &|_: &Address| ReadVerdict::Open) {
        descent::Entry::Unread { child, why } => Some((child, why)),
        descent::Entry::Descend(_) | descent::Entry::Passed => None,
    };
    let reason = Address::parse("lab/a:b").unwrap_err().recovery().to_owned();
    let why = format!("its name is not an address: {reason}");
    assert_eq!(named, Some(("lab/a:b".to_owned(), why)));
}

/// The test city's transcripts are one JSON object per line, tens of
/// kilobytes each: four whole lines spent the answer's byte budget and
/// the search stopped with every later transcript in the directory
/// unsearched. A hit carries a window of its line around the match, so
/// a directory of such files answers every hit.
#[test]
fn hits_in_long_lines_all_come_back_and_each_carries_its_match() {
    let dir = tempfile::tempdir().unwrap();
    let room = dir.path().join("hall").join("clerk");
    std::fs::create_dir_all(&room).unwrap();
    let filler = "x".repeat(30_000);
    for at in 0..12 {
        std::fs::write(
            room.join(format!("run-{at:02}.jsonl")),
            format!("{{\"text\":\"{filler} hall/mayor {filler}\"}}\n"),
        )
        .unwrap();
    }
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let outcome = tool
        .invoke(&call(&[
            ("text", Value::String("hall/mayor".to_owned())),
            ("path", Value::String("hall/clerk".to_owned())),
            ("context", Value::Number(1.into())),
        ]))
        .unwrap();
    let map = outcome.result.as_map();
    let matches = map["matches"].as_array().unwrap();
    assert_eq!(
        (map["count"].as_u64(), map["truncated"].as_bool()),
        (Some(12), Some(false))
    );
    assert!(
        matches
            .iter()
            .all(|hit| hit["text"].as_str().unwrap().contains("hall/mayor"))
    );
}

/// A search that stops at a limit says which limit and how to narrow,
/// because `truncated: true` alone reads as a fault of the tool.
#[test]
fn a_search_stopped_at_the_hit_cap_says_so_and_how_to_narrow() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab").join("Many.md"),
        "needle\n".repeat(100),
    )
    .unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let outcome = tool.invoke(&text("needle")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["truncated"], Value::Bool(true));
    let stopped = map
        .get("stopped")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        stopped.contains("64") && stopped.contains("path"),
        "stopped: {stopped:?}"
    );
}
