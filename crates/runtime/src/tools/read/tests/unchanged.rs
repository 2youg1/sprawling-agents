// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

use super::*;
use crate::tools::ReadShown;

fn reader(root: &Path) -> (ReadTool, ReadShown) {
    let shown = ReadShown::default();
    (tool(root).0.recalling(shown.clone()), shown)
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
    // Older than the race window: the repeat must not need the bytes.
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(10))
        .unwrap();
    let (reader, shown) = reader(dir.path());

    let first = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        first.result.as_map().get("text").and_then(Value::as_str),
        Some(text.as_str())
    );
    confirm(&shown, "call-1");
    let second = reader.invoke(&call("work/long.txt")).unwrap();
    let answer = second.result.as_map();

    assert_eq!(answer.get("unchanged"), Some(&Value::Bool(true)));
    assert_eq!(answer.get("text"), None);
    assert_eq!(answer.get("bytes"), Some(&Value::Number(text.len().into())));
    assert_eq!(answer.get("total_lines"), Some(&Value::Number(400.into())));
    assert_eq!(answer.get("next_offset"), None);
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
    let (reader, shown) = reader(dir.path());
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
