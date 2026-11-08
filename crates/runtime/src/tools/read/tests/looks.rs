// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::tools::{EditDesk, EditTool, PathLocks};

/// A read and an edit sharing one run's table, the way the workbench
/// lays them out.
fn run(root: &Path) -> (ReadTool, EditTool, ReadShown) {
    let seen = ReadStamps::default();
    let shown = ReadShown::default();
    let (reader, _catalog) = tool(root);
    let reader = reader
        .recalling(shown.clone())
        .clocking(Arc::new(|| Ok(TimeMs::new(u64::MAX))));
    let work = kernel::Address::parse("work").unwrap();
    let editor = EditTool::new(
        root,
        work.clone(),
        kernel::WriteDomain::new(vec![work]).unwrap(),
        EditDesk {
            policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)).reader(),
            seen: seen.clone(),
            locks: PathLocks::default(),
        },
    )
    .unwrap();
    (reader.noting(seen), editor, shown)
}

fn edit(path: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (key, value) in [
        ("file_path", path),
        ("old_string", old),
        ("new_string", new),
    ] {
        args.insert(key.to_owned(), Value::String(value.to_owned()));
    }
    ToolCall {
        id: "call-2".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

/// A read answers no version and hashes nothing; what it leaves behind
/// is the run's stamp, and that is what admits the edit
/// (`crates/runtime/spec/Tools/Edit.lean` §8-64).
#[test]
fn a_whole_read_admits_the_runs_edit_and_answers_no_version() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work").join("a.txt"), "one\n").unwrap();
    let (reader, editor, _) = run(dir.path());

    let refused = editor
        .invoke(&edit("work/a.txt", "one", "two"))
        .unwrap_err();
    let answer = reader.invoke(&call("work/a.txt")).unwrap();
    let landed = editor.invoke(&edit("work/a.txt", "one", "two"));

    assert_eq!(
        (
            *refused.code(),
            answer.result.as_map().contains_key("version"),
            landed.is_ok(),
            std::fs::read_to_string(dir.path().join("work").join("a.txt")).unwrap(),
        ),
        (AxCode::InvalidArgs, false, true, "two\n".to_owned())
    );
}

/// A whole read the line cap truncated is not a look - the caller
/// believes it saw the file - and a window the caller asked for is.
#[test]
fn a_truncated_whole_read_is_not_a_look_and_an_asked_window_is() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let long: String = (0..600).map(|n| format!("line {n}\n")).collect();
    std::fs::write(dir.path().join("work").join("long.txt"), &long).unwrap();
    let (reader, editor, _) = run(dir.path());

    reader.invoke(&call("work/long.txt")).unwrap();
    let after_truncated = editor.invoke(&edit("work/long.txt", "line 599\n", "last\n"));
    reader
        .invoke(&interval("work/long.txt", 590, Some(10)))
        .unwrap();
    let after_window = editor.invoke(&edit("work/long.txt", "line 599\n", "last\n"));

    assert_eq!(
        (
            after_truncated.map(|_| ()).map_err(|err| *err.code()),
            after_window.map(|_| ()).map_err(|err| *err.code()),
        ),
        (Err(AxCode::InvalidArgs), Ok(()))
    );
}

/// S3 and the durability case together: an edit invalidates the old
/// window evidence even though `seen` now holds the edit's new stamp;
/// once the new full text is confirmed, a later repeat may use a stub.
#[test]
fn read_edit_read_returns_the_new_text_before_a_later_repeat_can_be_a_stub() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let original: String = (0..400)
        .map(|line| format!("line {line}: a substantial passage\n"))
        .collect();
    std::fs::write(dir.path().join("work/long.txt"), &original).unwrap();
    let (reader, editor, shown) = run(dir.path());

    reader.invoke(&call("work/long.txt")).unwrap();
    shown.admit("call-1");
    shown.confirm(&std::collections::BTreeSet::new());
    editor
        .invoke(&edit(
            "work/long.txt",
            "line 10: a substantial passage\n",
            "LINE 10: the edited passage\n",
        ))
        .unwrap();
    let after_edit = reader.invoke(&call("work/long.txt")).unwrap();
    let changed = after_edit
        .result
        .as_map()
        .get("text")
        .and_then(Value::as_str)
        .unwrap();

    assert!(changed.contains("LINE 10: the edited passage"));
    assert!(!changed.contains("line 10: a substantial passage"));
    assert_eq!(after_edit.result.as_map().get("unchanged"), None);
    shown.admit("call-1");
    shown.confirm(&std::collections::BTreeSet::new());
    let repeated = reader.invoke(&call("work/long.txt")).unwrap();
    assert_eq!(
        repeated.result.as_map().get("unchanged"),
        Some(&Value::Bool(true))
    );
}
