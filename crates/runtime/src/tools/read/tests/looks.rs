// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::tools::{EditDesk, EditTool, PathLocks};

/// A read and an edit sharing one run's table, the way the workbench
/// lays them out.
fn run(root: &Path) -> (ReadTool, EditTool) {
    let seen = ReadStamps::default();
    let (reader, _catalog) = tool(root);
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
    (reader.noting(seen), editor)
}

fn edit(path: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (key, value) in [("path", path), ("old", old), ("new", new)] {
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
    let (reader, editor) = run(dir.path());

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
    let (reader, editor) = run(dir.path());

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
