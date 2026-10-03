// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

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

/// One hit whose own line is wider than the window is cut to the window
/// and marked, instead of travelling whole: a single line of a generated
/// file would otherwise spend the answer's budget by itself.
#[test]
fn a_hit_on_a_line_wider_than_the_window_is_cut_to_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let wide = format!(
        "the ledger {}
",
        "x".repeat(cap.saturating_mul(2))
    );
    std::fs::write(dir.path().join("lab").join("Wide.md"), wide).unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool.invoke(&text("the ledger")).unwrap();
    let map = outcome.result.as_map();
    let hit = map["matches"][0]["text"].as_str().unwrap();
    assert!(
        hit.len() <= 2 * hit::LINE_CAP,
        "{} bytes came back",
        hit.len()
    );
    assert!(hit.starts_with("the ledger"));
    assert!(hit.ends_with(" bytes]"), "the cut is marked");
    assert_eq!(map["truncated"], Value::Bool(false));
}
