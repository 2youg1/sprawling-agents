// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::*;
use crate::tools::ReadShown;

fn reader(root: &Path) -> (ReadTool, ReadShown) {
    let shown = ReadShown::default();
    let reader = tool(root)
        .0
        .recalling(shown.clone())
        .clocking(Arc::new(|| Ok(TimeMs::new(u64::MAX))));
    (reader, shown)
}

fn long_text() -> String {
    (0..400)
        .map(|line| format!("line {line}: a substantial passage\n"))
        .collect()
}

fn confirm(shown: &ReadShown, call: &str) {
    shown.admit(call);
    shown.confirm(&BTreeSet::new());
}

#[test]
fn a_confirmed_repeat_returns_unchanged_instead_of_the_text() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let path = dir.path().join("work/long.txt");
    let text = long_text();
    std::fs::write(&path, &text).unwrap();
    let (reader, shown) = reader(dir.path());

    let first = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        first.result.as_map().get("text").and_then(Value::as_str),
        Some(text.as_str())
    );
    confirm(&shown, "call-1");
    let second = reader.invoke(&call("work/long.txt")).unwrap();
    let answer = second.result.as_map();

    if !cfg!(any(windows, target_os = "linux", target_os = "macos")) {
        assert_eq!(answer.get("unchanged"), None);
        assert_eq!(
            answer.get("text").and_then(Value::as_str),
            Some(text.as_str())
        );
        return;
    }
    assert_eq!(answer.get("unchanged"), Some(&Value::Bool(true)));
    assert_eq!(answer.get("text"), None);
    assert_eq!(answer.get("bytes"), None);
    assert_eq!(answer.get("total_lines"), Some(&Value::Number(400.into())));
    assert_eq!(answer.get("next_offset"), None);
    assert_eq!(answer.get("offset"), Some(&Value::Number(0.into())));
}

#[test]
fn a_small_file_does_not_pay_for_an_unchanged_stub() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work/a.txt"), "one\ntwo\n").unwrap();
    let (reader, shown) = reader(dir.path());

    reader.invoke(&call("work/a.txt")).unwrap();
    confirm(&shown, "call-1");
    let second = reader.invoke(&call("work/a.txt")).unwrap();

    assert_eq!(
        second.result.as_map().get("text").and_then(Value::as_str),
        Some("one\ntwo\n")
    );
    assert_eq!(second.result.as_map().get("unchanged"), None);
}

#[test]
fn a_same_length_same_mtime_rewrite_is_compared_by_content() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let path = dir.path().join("work/long.txt");
    let original = long_text();
    std::fs::write(&path, &original).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let shown = ReadShown::default();
    // No clock is conservative: the first claim must carry a digest.
    let reader = tool(dir.path()).0.recalling(shown.clone());
    reader.invoke(&call("work/long.txt")).unwrap();
    confirm(&shown, "call-1");

    let changed = original.replacen("line 0:", "LINE 0:", 1);
    assert_eq!(changed.len(), original.len());
    std::fs::write(&path, changed.as_bytes()).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let second = reader.invoke(&call("work/long.txt")).unwrap();

    assert_eq!(
        second.result.as_map().get("text").and_then(Value::as_str),
        Some(changed.as_str())
    );
    assert_eq!(second.result.as_map().get("unchanged"), None);
}

#[test]
fn speculation_and_shortening_do_not_create_window_evidence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work/long.txt"), long_text()).unwrap();
    let (reader, shown) = reader(dir.path());

    reader.invoke(&call("work/long.txt")).unwrap();
    let speculative = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(speculative.result.as_map().get("unchanged"), None);

    shown.admit("call-1");
    shown.confirm(&BTreeSet::from(["call-1".to_owned()]));
    let shortened = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(shortened.result.as_map().get("unchanged"), None);
    assert!(shortened.result.as_map().contains_key("text"));
}

#[test]
fn coverage_follows_the_actual_window_not_the_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work/long.txt"), long_text()).unwrap();
    let (reader, shown) = reader(dir.path());

    reader.invoke(&call("work/long.txt")).unwrap();
    confirm(&shown, "call-1");
    let window = reader
        .invoke(&interval("work/long.txt", 100, Some(20)))
        .unwrap();
    assert_eq!(
        window.result.as_map().get("unchanged"),
        cfg!(any(windows, target_os = "linux", target_os = "macos")).then_some(&Value::Bool(true))
    );
    if cfg!(any(windows, target_os = "linux", target_os = "macos")) {
        assert_eq!(
            window.result.as_map().get("lines"),
            Some(&Value::Number(20.into()))
        );
        assert_eq!(
            window.result.as_map().get("total_lines"),
            Some(&Value::Number(400.into()))
        );
    }
    let (window_reader, window_shown) = super::unchanged::reader(dir.path());
    window_reader
        .invoke(&interval("work/long.txt", 100, Some(20)))
        .unwrap();
    confirm(&window_shown, "call-1");
    let whole = window_reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(whole.result.as_map().get("unchanged"), None);
    assert!(whole.result.as_map().contains_key("text"));
}

#[test]
fn a_truncated_whole_read_never_becomes_whole_file_evidence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let text: String = (0..600)
        .map(|line| format!("line {line}: a substantial passage\n"))
        .collect();
    std::fs::write(dir.path().join("work/long.txt"), text).unwrap();
    let (reader, shown) = reader(dir.path());

    let first = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        first.result.as_map().get("next_offset"),
        Some(&Value::Number(512.into()))
    );
    confirm(&shown, "call-1");
    let second = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(second.result.as_map().get("unchanged"), None);
    assert_eq!(
        second.result.as_map().get("next_offset"),
        Some(&Value::Number(512.into()))
    );
}

#[cfg(unix)]
#[test]
fn a_relative_symlink_reuses_evidence_at_the_admitted_address() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work/long.txt"), long_text()).unwrap();
    std::os::unix::fs::symlink("long.txt", dir.path().join("work/link.txt")).unwrap();
    let (reader, shown) = reader(dir.path());
    reader.invoke(&call("work/link.txt")).unwrap();
    confirm(&shown, "call-1");
    let repeated = reader.invoke(&call("work/link.txt")).unwrap();
    assert_eq!(
        repeated.result.as_map().get("unchanged"),
        cfg!(any(target_os = "linux", target_os = "macos")).then_some(&Value::Bool(true))
    );
}

#[test]
fn the_clock_is_sampled_before_reading_the_body() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let path = dir.path().join("work/long.txt");
    std::fs::write(&path, "before clock\n").unwrap();
    let reader = tool(dir.path()).0.clocking(Arc::new(move || {
        std::fs::write(&path, "after clock\n").unwrap();
        Ok(TimeMs::new(u64::MAX))
    }));
    let answer = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        answer.result.as_map().get("text").and_then(Value::as_str),
        Some("after clock\n")
    );
}

#[test]
fn a_failed_retry_with_the_same_call_id_discards_the_earlier_claim() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let text = long_text();
    std::fs::write(dir.path().join("work/long.txt"), &text).unwrap();
    let (reader, shown) = reader(dir.path());
    reader.invoke(&call("work/long.txt")).unwrap();
    let mut retry = call("work/long.txt");
    retry.args = Payload::empty();
    assert!(reader.invoke(&retry).is_err());
    confirm(&shown, "call-1");
    let answer = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        answer.result.as_map().get("text").and_then(Value::as_str),
        Some(text.as_str())
    );
}
