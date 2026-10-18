// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a read promises about a file larger than one window: the answer
//! is the interval it was asked for, the fields it cannot know yet are
//! absent rather than wrong, and the next read of the same file carries
//! them (`crates/runtime/Spec.lean` §8-29-7, D10).

use super::*;

/// A file of `lines` lines, about forty bytes each, so a window of 512
/// lines is a small part of it.
fn long_body(lines: usize) -> String {
    (0..lines)
        .map(|n| format!("{n:06} the quick brown fox jumps over a lazy dog\n"))
        .collect()
}

/// A file past one window: the first answer is the cap and the offset,
/// and it says nothing it has not read.
#[test]
fn a_windowed_read_answers_the_cap_and_defers_what_it_did_not_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(4_000);
    std::fs::write(dir.path().join("work").join("Long.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let first = reader.invoke(&call("work/Long.md")).unwrap();
    let map = first.result.as_map();
    assert_eq!(map["text"].as_str().unwrap().lines().count(), 512);
    assert_eq!(map["next_offset"], 512);
    assert!(
        map.get("version").is_none(),
        "a window answers no version: half a file's hash cannot guard `edit`"
    );
    assert!(
        map.get("total_lines").is_none(),
        "a window answers no total: counting one means scanning the file"
    );
}

/// The facts the first answer deferred arrive without a second window
/// being cut: the same file, read again, carries the whole-file version
/// and the total.
#[test]
fn the_next_read_of_the_same_file_carries_what_the_window_deferred() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(4_000);
    std::fs::write(dir.path().join("work").join("Long.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let _first = reader.invoke(&call("work/Long.md")).unwrap();
    // The fill runs behind the answer; this waits for it rather than
    // racing it.
    let second = with_deferred_facts(&reader, "work/Long.md");
    let map = second.result.as_map();
    let version = map["version"].as_str().unwrap_or_default();
    kernel::Locator::parse(version).unwrap();
    assert_eq!(map["total_lines"], 4_000);
    assert_eq!(map["next_offset"], 512);
}

/// A ranged read of a file the index knows lands on the line it was
/// asked for, and reports the same total as a read from the head.
#[test]
fn a_ranged_read_of_an_indexed_file_starts_where_the_seek_table_says() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(4_000);
    std::fs::write(dir.path().join("work").join("Long.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let _first = reader.invoke(&call("work/Long.md")).unwrap();
    let _warm = with_deferred_facts(&reader, "work/Long.md");

    let ranged = reader
        .invoke(&interval("work/Long.md", 1_000, None))
        .unwrap();
    let map = ranged.result.as_map();
    let text = map["text"].as_str().unwrap();
    assert!(
        text.starts_with("001000 the quick brown fox"),
        "the seek landed on line 1000, not on the window's start"
    );
    assert_eq!(text.lines().count(), 512);
    assert_eq!(map["total_lines"], 4_000);
    assert_eq!(map["next_offset"], 1_512);
}

/// Reading past the end of a file the index knows answers empty with the
/// total, and never asks the disk for bytes that are not there.
#[test]
fn an_offset_past_the_end_of_an_indexed_file_still_names_the_total() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(4_000);
    std::fs::write(dir.path().join("work").join("Long.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let _first = reader.invoke(&call("work/Long.md")).unwrap();
    let _warm = with_deferred_facts(&reader, "work/Long.md");

    let past = reader
        .invoke(&interval("work/Long.md", 9_000, None))
        .unwrap();
    let map = past.result.as_map();
    assert_eq!(map["text"], "");
    assert_eq!(map["total_lines"], 4_000);
    assert!(map.get("next_offset").is_none());
}

/// A file that fits one window is answered whole, exactly as before this
/// change: its total is the answer itself, so the field is absent, and
/// its version is present because the window held every byte of it.
#[test]
fn a_file_inside_one_window_keeps_the_answer_it_had() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(20);
    std::fs::write(dir.path().join("work").join("Short.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let outcome = reader.invoke(&call("work/Short.md")).unwrap();
    let map = outcome.result.as_map();
    assert_eq!(map["text"].as_str().unwrap(), body);
    assert_eq!(map["bytes"], body.len());
    assert!(map.get("total_lines").is_none());
    assert!(map.get("next_offset").is_none());
    let version = map["version"].as_str().unwrap_or_default();
    assert_eq!(
        version,
        crate::tools::version_of(body.as_bytes()),
        "a whole window is the whole file, so its version costs nothing"
    );
}

/// The version a ranged read reports is the whole file's, whatever
/// interval was cut: `edit` guards the file, not the lines.
#[test]
fn a_ranged_answer_names_the_whole_files_version() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let body = long_body(20);
    std::fs::write(dir.path().join("work").join("Short.md"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let _whole = reader.invoke(&call("work/Short.md")).unwrap();
    let ranged = reader
        .invoke(&interval("work/Short.md", 5, Some(3)))
        .unwrap();
    let map = ranged.result.as_map();
    assert_eq!(map["version"].as_str().unwrap_or_default(), {
        crate::tools::version_of(body.as_bytes())
    });
    assert_eq!(map["total_lines"], 20);
    assert_eq!(map["next_offset"], 8);
}

/// The fill runs on its own thread, so waiting for a fact it produces is
/// a poll of the answer rather than a sleep in the test.
fn with_deferred_facts(reader: &ReadTool, path: &str) -> ToolOutcome {
    // The clock is the city's to sample, not a test's (AGENTS.md, one
    // sampling point), so waiting is a bounded count of reads.
    const ATTEMPTS: usize = 2_000;
    for _ in 0..ATTEMPTS {
        let outcome = reader.invoke(&call(path)).unwrap();
        if outcome.result.as_map().contains_key("version") {
            return outcome;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the fill behind the first answer never arrived for {path}");
}
