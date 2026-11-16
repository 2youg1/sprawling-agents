// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_passage_copied_with_its_line_marks_edits_the_lines_it_marks() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..12).map(|n| format!("row {n}\n")).collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");

    tool.invoke(&call(
        "work/a.txt",
        "row 8\n10|row 9\n",
        "row 8\n10|row nine\n",
    ))
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 9\n", "row nine\n")
    );
}

#[test]
fn a_passage_that_matches_as_it_came_keeps_what_looks_like_a_mark() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/t.md"), "| a |\n12|b\n").unwrap();
    look(&tool, tmp.path(), "work/t.md");
    tool.invoke(&call("work/t.md", "12|b\n", "12|c\n")).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/t.md")).unwrap(),
        "| a |\n12|c\n"
    );
}

#[test]
fn a_numbered_line_added_in_the_replacement_is_written_as_it_came() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..12).map(|n| format!("row {n}\n")).collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    tool.invoke(&call("work/a.txt", "10|row 9\n", "10|row nine\n12|keep\n"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 9\n", "row nine\n12|keep\n")
    );
}

#[test]
fn a_copied_passage_keeps_a_numbered_line_the_file_holds() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..13)
        .map(|n| match n {
            10 => "7|literal\n".to_owned(),
            _ => format!("row {n}\n"),
        })
        .collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    tool.invoke(&call(
        "work/a.txt",
        "10|row 9\n7|literal\n",
        "10|row 9\n7|changed\n",
    ))
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("7|literal\n", "7|changed\n")
    );
}

#[test]
fn a_passage_copied_from_a_search_hit_with_context_edits_the_lines_it_marks() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..12).map(|n| format!("row {n}\n")).collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let block = crate::rendering::shown(
        crate::rendering::CURRENT,
        &kernel::ToolName::parse("search").unwrap(),
        &kernel::event::record::ToolAnswer::Answered {
            result: kernel::Payload::of(&serde_json::json!({
                "matches": [{"path": "work/a.txt", "line": 7, "from": 5,
                             "text": "row 5\nrow 6\nrow 7\nrow 8"}],
                "count": 1, "truncated": false, "unreadable": 0, "unread": []
            }))
            .unwrap(),
        },
    )
    .unwrap();
    let old: String = block
        .lines()
        .skip(2)
        .take(3)
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(old, "6|row 5\nrow 6\nrow 7\n");
    let new = old.replace("row 7", "row seven");
    tool.invoke(&call("work/a.txt", &old, &new)).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 7\n", "row seven\n")
    );
}
