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

#[test]
fn an_empty_file_has_no_continuation_before_or_after_indexing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.txt"), "").unwrap();
    let (reader, _catalog) = tool(dir.path());
    let expected = serde_json::json!({
        "path": "empty.txt", "text": "", "bytes": 0, "total_lines": 0,
        "version": crate::tools::version_of(b""),
    });

    for _ in 0..2 {
        let answer = reader.invoke(&call("empty.txt")).unwrap();
        assert_eq!(answer.result.as_map(), expected.as_object().unwrap());
    }
}

#[test]
fn exactly_the_line_cap_is_a_complete_file_with_or_without_a_final_newline() {
    let dir = tempfile::tempdir().unwrap();
    for final_line in ["last\n", "last"] {
        let body = format!("{}{final_line}", "line\n".repeat(511));
        std::fs::write(dir.path().join("exact.txt"), &body).unwrap();
        let (reader, _catalog) = tool(dir.path());

        let answer = reader.invoke(&call("exact.txt")).unwrap();
        assert_eq!(
            answer.result.as_map(),
            serde_json::json!({
                "path": "exact.txt", "text": body, "bytes": body.len(),
                "version": crate::tools::version_of(body.as_bytes()),
            })
            .as_object()
            .unwrap(),
        );
    }
}

#[test]
fn indexed_reads_preserve_blank_crlf_and_unicode_lines_at_sector_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let body = format!("{}\n\na\r\né\ntail", "x".repeat(4_095));
    std::fs::write(dir.path().join("lines.txt"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());
    reader.invoke(&call("lines.txt")).unwrap();

    for (offset, text, next) in [
        (1, "\n", Some(2)),
        (2, "a\r\n", Some(3)),
        (3, "é\n", Some(4)),
        (4, "tail", None),
    ] {
        let answer = reader
            .invoke(&interval("lines.txt", offset, Some(1)))
            .unwrap();
        let mut expected = serde_json::json!({
            "path": "lines.txt", "text": text, "bytes": text.len(),
            "version": crate::tools::version_of(body.as_bytes()),
        });
        if let Some(next) = next {
            expected["total_lines"] = serde_json::json!(5);
            expected["next_offset"] = serde_json::json!(next);
        }
        assert_eq!(answer.result.as_map(), expected.as_object().unwrap());
    }
}

#[test]
fn offsets_at_the_end_and_u64_max_answer_empty_with_and_without_an_index() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("short.txt"), "one\ntwo").unwrap();
    for indexed in [false, true] {
        for offset in [2, u64::MAX] {
            let (reader, _catalog) = tool(dir.path());
            if indexed {
                reader.invoke(&call("short.txt")).unwrap();
            }
            let answer = reader.invoke(&interval("short.txt", offset, None)).unwrap();
            let mut expected = serde_json::json!({
                "path": "short.txt", "text": "", "bytes": 0, "total_lines": 2,
            });
            if indexed {
                expected["version"] = serde_json::json!(crate::tools::version_of(b"one\ntwo"));
            }
            assert_eq!(answer.result.as_map(), expected.as_object().unwrap());
        }
    }
}

#[test]
fn a_utf8_character_split_across_chunks_is_preserved_in_a_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let chunk = kernel::consts_policy::INTERVAL_CAP_BYTES / 2;
    let body = format!("{}é\r\n\n終", "x".repeat(chunk.saturating_sub(1)));
    std::fs::write(dir.path().join("unicode.txt"), &body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let answer = reader.invoke(&call("unicode.txt")).unwrap();
    assert_eq!(
        answer.result.as_map(),
        serde_json::json!({
            "path": "unicode.txt", "text": body, "bytes": body.len(),
            "version": crate::tools::version_of(body.as_bytes()),
        })
        .as_object()
        .unwrap(),
    );
}

#[test]
fn an_unindexed_range_discards_lines_spanning_multiple_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let body = format!("{}\n\n終", "x".repeat(cap.saturating_add(17)));
    std::fs::write(dir.path().join("range.txt"), body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let answer = reader.invoke(&interval("range.txt", 1, None)).unwrap();
    assert_eq!(
        answer.result.as_map(),
        serde_json::json!({"path": "range.txt", "text": "\n終", "bytes": 4})
            .as_object()
            .unwrap(),
    );
}

#[test]
fn invalid_utf8_outside_the_requested_lines_does_not_refuse_a_text_window() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mixed.txt"), b"\xff\nvalid\n\xfe\n").unwrap();
    let (reader, _catalog) = tool(dir.path());

    let answer = reader.invoke(&interval("mixed.txt", 1, Some(1))).unwrap();
    assert_eq!(
        answer.result.as_map(),
        serde_json::json!({
            "path": "mixed.txt", "text": "valid\n", "bytes": 6,
            "total_lines": 3, "next_offset": 2,
        })
        .as_object()
        .unwrap(),
    );
}

#[test]
fn invalid_utf8_in_the_requested_window_is_an_argument_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mixed.txt"), b"valid\n\xff\n").unwrap();
    let (reader, _catalog) = tool(dir.path());

    let err = reader
        .invoke(&interval("mixed.txt", 1, Some(1)))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.nearby().is_empty());
    assert!(err.recovery().contains("UTF-8"));
}

#[test]
fn a_line_that_exactly_spends_the_byte_budget_does_not_take_the_following_tail() {
    let dir = tempfile::tempdir().unwrap();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let first = format!("{}\n", "x".repeat(cap.saturating_sub(1)));
    for tail in ["tail\n", "tail"] {
        std::fs::write(dir.path().join("budget.txt"), format!("{first}{tail}")).unwrap();
        let (reader, _catalog) = tool(dir.path());

        let answer = reader.invoke(&call("budget.txt")).unwrap();
        let map = answer.result.as_map();
        assert_eq!(
            (
                map["text"].as_str(),
                map["bytes"].as_u64(),
                map.get("next_offset"),
            ),
            (
                Some(first.as_str()),
                Some(u64::try_from(cap).unwrap()),
                Some(&serde_json::json!(1)),
            ),
            "tail {tail:?} crossed the byte budget",
        );
    }
}

#[test]
fn a_version_on_a_short_truncated_answer_still_hashes_the_whole_file() {
    let dir = tempfile::tempdir().unwrap();
    let body = "one\ntwo\nthree\n";
    std::fs::write(dir.path().join("version.txt"), body).unwrap();
    let expected = crate::tools::version_of(body.as_bytes());
    for indexed in [false, true] {
        let (reader, _catalog) = tool(dir.path());
        if indexed {
            reader.invoke(&call("version.txt")).unwrap();
        }
        let answer = reader.invoke(&interval("version.txt", 0, Some(1))).unwrap();
        if let Some(version) = answer.result.as_map().get("version") {
            assert_eq!(version.as_str(), Some(expected.as_str()));
        } else {
            assert!(
                !indexed,
                "an indexed file already has its whole-file version"
            );
        }
    }
}

#[test]
fn catalog_intervals_start_at_the_requested_line_and_terminate_at_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let (reader, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_carried_skill(CatalogEntry {
            name: "carried".to_owned(),
            disclosure: "a carried document".to_owned(),
            expansion: "α\r\n\nend".to_owned(),
            hash: None,
            package: None,
        })
        .unwrap();

    for (offset, limit, text, total, next) in [
        (0, 1, "α\r\n", Some(3), Some(1)),
        (1, 1, "\n", Some(3), Some(2)),
        (2, 10, "end", None, None),
        (3, 10, "", Some(3), None),
        (u64::MAX, 10, "", Some(3), None),
    ] {
        let answer = reader
            .invoke(&interval("carried", offset, Some(limit)))
            .unwrap();
        let mut expected = serde_json::json!({
            "path": "carried", "text": text, "bytes": text.len(),
        });
        if let Some(total) = total {
            expected["total_lines"] = serde_json::json!(total);
        }
        if let Some(next) = next {
            expected["next_offset"] = serde_json::json!(next);
        }
        assert_eq!(answer.result.as_map(), expected.as_object().unwrap());
    }
}

#[test]
fn a_changed_size_or_stamp_invalidates_both_continuation_bytes_and_index_facts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("changed.txt");
    for (replacement, stamp) in [
        ("new!\nnext\n", 1_600_000_002),
        ("bigger!\nchanged\n", 1_600_000_000),
    ] {
        let write = |body: &str, stamp: u64| {
            std::fs::write(&path, body).unwrap();
            std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(
                    std::time::UNIX_EPOCH
                        .checked_add(std::time::Duration::from_secs(stamp))
                        .unwrap(),
                )
                .unwrap();
        };
        write("old!\nrest\n", 1_600_000_000);
        let (reader, _catalog) = tool(dir.path());
        // A whole read offers the index entry synchronously, and the
        // truncated read after it holds the continuation bytes: both
        // caches stand before the rewrite, with no fill to race.
        let whole = reader.invoke(&call("changed.txt")).unwrap();
        assert!(whole.result.as_map().contains_key("version"));
        reader.invoke(&interval("changed.txt", 0, Some(1))).unwrap();
        write(replacement, stamp);

        let answer = reader.invoke(&interval("changed.txt", 1, None)).unwrap();
        let text = replacement.split_inclusive('\n').nth(1).unwrap();
        assert_eq!(
            answer.result.as_map(),
            serde_json::json!({"path": "changed.txt", "text": text, "bytes": text.len()})
                .as_object()
                .unwrap(),
        );
    }
}

/// The version of a short file read with a limit below its line count
/// is the whole file's or absent, never the delivered prefix's.
#[test]
fn a_limit_below_a_short_files_line_count_offers_no_prefix_version() {
    let dir = tempfile::tempdir().unwrap();
    let body = "first\nsecond\n";
    std::fs::write(dir.path().join("short.txt"), body).unwrap();
    let (reader, _catalog) = tool(dir.path());

    let answer = reader.invoke(&interval("short.txt", 0, Some(1))).unwrap();
    let map = answer.result.as_map();
    assert_eq!(map["text"], "first\n");
    if let Some(version) = map.get("version") {
        assert_eq!(
            version.as_str(),
            Some(crate::tools::version_of(body.as_bytes()).as_str())
        );
    }
    let whole = with_deferred_facts(&reader, "short.txt");
    assert_eq!(
        whole.result.as_map()["version"].as_str(),
        Some(crate::tools::version_of(body.as_bytes()).as_str())
    );
}

/// An indexed file that is written again gets its facts back: the stale
/// entry is refilled, not kept as a reason to never fill again.
#[test]
fn an_edited_indexed_file_regains_its_deferred_facts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let path = dir.path().join("work").join("Long.md");
    std::fs::write(&path, long_body(4_000)).unwrap();
    let (reader, _catalog) = tool(dir.path());
    let _first = reader.invoke(&call("work/Long.md")).unwrap();
    let _warm = with_deferred_facts(&reader, "work/Long.md");

    let edited = long_body(4_001);
    std::fs::write(&path, &edited).unwrap();
    let expected = crate::tools::version_of(edited.as_bytes());
    let again = with_deferred_facts(&reader, "work/Long.md");
    let map = again.result.as_map();
    assert_eq!(map["version"].as_str(), Some(expected.as_str()));
    assert_eq!(map["total_lines"], 4_001);
}

/// A file rewritten under the same size and mtime moves its line
/// starts; a ranged read through the stale seek table still answers the
/// lines it was asked for.
#[test]
fn a_stale_seek_table_under_the_same_size_and_stamp_answers_the_asked_lines() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let path = dir.path().join("work").join("Long.md");
    let write = |body: &str| {
        std::fs::write(&path, body).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(
                std::time::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(1_600_000_000))
                    .unwrap(),
            )
            .unwrap();
    };
    let old = long_body(4_000);
    write(&old);
    let (reader, _catalog) = tool(dir.path());
    let _first = reader.invoke(&call("work/Long.md")).unwrap();
    let _warm = with_deferred_facts(&reader, "work/Long.md");

    // One byte more on the first line and one fewer on the last: the
    // same size, every later line one byte further on.
    let mut shifted = format!("x{old}");
    shifted.truncate(shifted.len() - 2);
    shifted.push('\n');
    assert_eq!(shifted.len(), old.len());
    write(&shifted);

    let answer = reader
        .invoke(&interval("work/Long.md", 1_000, Some(3)))
        .unwrap();
    let expected: String = shifted.split_inclusive('\n').skip(1_000).take(3).collect();
    assert_eq!(
        answer.result.as_map()["text"].as_str(),
        Some(expected.as_str())
    );
}
