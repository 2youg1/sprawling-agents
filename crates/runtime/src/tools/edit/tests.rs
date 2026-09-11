// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::Address;

fn tool(root: &Path) -> EditTool {
    let work = Address::parse("work").unwrap();
    let domain = kernel::WriteDomain::new(vec![work.clone()]).unwrap();
    std::fs::create_dir_all(root.join("work")).unwrap();
    EditTool::new(root, work, domain).unwrap()
}

fn call(path: &str, base: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (k, v) in [
        ("path", path),
        ("base_version", base),
        ("old", old),
        ("new", new),
    ] {
        args.insert(k.to_owned(), Value::String(v.to_owned()));
    }
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

#[test]
fn an_edit_against_the_version_it_saw_lands_and_echoes_a_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "one\ntwo\nthree\n").unwrap();
    let version = version_of(b"one\ntwo\nthree\n");
    let outcome = tool
        .invoke(&call("work/a.txt", &version, "two", "TWO"))
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "one\nTWO\nthree\n"
    );
    let diff = result["diff"].as_str().unwrap();
    assert!(diff.contains("-two"), "{diff}");
    assert!(diff.contains("+TWO"), "{diff}");
    assert_eq!(
        result["new_version"].as_str().unwrap(),
        version_of(b"one\nTWO\nthree\n")
    );
}

#[test]
fn a_file_that_moved_refuses_and_names_the_version_it_is_at_now() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "current\n").unwrap();
    let err = match tool.invoke(&call("work/a.txt", "0000000000000000", "current", "next")) {
        Err(err) => err,
        Ok(_) => panic!("a stale base_version must refuse"),
    };
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(err.subject().contains(&version_of(b"current\n")));
    // The file is untouched: a refused edit changes nothing.
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "current\n"
    );
}

#[test]
fn base_version_new_creates_the_file_and_parents() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    let outcome = tool
        .invoke(&call("work/room/notes.md", "new", "", "first line\n"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/room/notes.md")).unwrap(),
        "first line\n"
    );
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(
        result["new_version"].as_str().unwrap(),
        version_of(b"first line\n")
    );
    assert!(result["diff"].as_str().unwrap().contains("+first line"));
}

#[test]
fn creating_over_an_existing_file_is_a_version_conflict_naming_the_real_version() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "already\n").unwrap();
    let err = match tool.invoke(&call("work/a.txt", "new", "", "other\n")) {
        Err(err) => err,
        Ok(_) => panic!("an existing file must refuse creation"),
    };
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(err.subject().contains(&version_of(b"already\n")));
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "already\n"
    );
}

#[test]
fn creating_with_a_nonempty_old_is_refused_with_the_form_to_use() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    let err = match tool.invoke(&call("work/b.txt", "new", "something", "content")) {
        Err(err) => err,
        Ok(_) => panic!("a create replaces nothing"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(err.recovery().contains("old:\"\""));
}

#[test]
fn a_path_outside_the_write_domain_is_refused_before_the_disk_is_touched() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("outside.txt"), "x").unwrap();
    let mut tool = tool(tmp.path());
    for hostile in ["outside.txt", "elsewhere/f.md", ".sprawling/ledger/x"] {
        let err = match tool.invoke(&call(hostile, "new", "", "y")) {
            Err(err) => err,
            Ok(_) => panic!("{hostile} must be refused"),
        };
        assert_eq!(*err.code(), AxCode::OutsideWriteDomain, "{hostile}");
        // The three-part refusal comes from the kernel gate now, so the
        // way out is in its alternative rather than in a recovery line.
        let alternative = err
            .gate()
            .map(|gate| gate.alternative().to_owned())
            .unwrap_or_default();
        assert!(alternative.contains("work"), "{hostile}: {err}");
    }
    for illegal in ["../evil.txt", "/abs.txt", "work//x"] {
        let err = match tool.invoke(&call(illegal, "new", "", "y")) {
            Err(err) => err,
            Ok(_) => panic!("{illegal} must be refused"),
        };
        assert_eq!(*err.code(), AxCode::InvalidArgs, "{illegal}");
    }
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("outside.txt")).unwrap(),
        "x"
    );
}

#[test]
fn a_missing_file_names_the_create_form_in_its_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    let version = version_of(b"whatever");
    let err = match tool.invoke(&call("work/absent.md", &version, "a", "b")) {
        Err(err) => err,
        Ok(_) => panic!("a missing file must refuse a non-create edit"),
    };
    assert!(err.recovery().contains("base_version:\"new\""), "{err}");
}

#[test]
fn zero_or_many_matches_refuse_with_the_count() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "x\nx\n").unwrap();
    let version = version_of(b"x\nx\n");
    let err = match tool.invoke(&call("work/a.txt", &version, "x", "y")) {
        Err(err) => err,
        Ok(_) => panic!("an ambiguous match must refuse"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(err.subject().contains("2 times"), "{}", err.subject());

    let err = match tool.invoke(&call("work/a.txt", &version, "absent", "y")) {
        Err(err) => err,
        Ok(_) => panic!("a missing match must refuse"),
    };
    assert!(err.subject().contains("0 times"), "{}", err.subject());
}

#[test]
fn a_call_for_another_tool_is_refused_not_routed() {
    let tmp = tempfile::tempdir().unwrap();
    let mut tool = tool(tmp.path());
    let mut wrong = call("work/a.txt", "v", "a", "b");
    wrong.name = ToolName::parse("exec").unwrap();
    let err = match tool.invoke(&wrong) {
        Err(err) => err,
        Ok(_) => panic!("identity is fail-closed"),
    };
    assert_eq!(*err.code(), AxCode::InvalidArgs);
}

#[test]
fn a_documents_tool_writes_markdown_and_refuses_code_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("hall")).unwrap();
    let hall = Address::parse("hall").unwrap();
    let domain = kernel::WriteDomain::documents(vec![hall.clone()]).unwrap();
    let mut tool =
        EditTool::new(tmp.path(), Address::parse("hall/mayor").unwrap(), domain).unwrap();
    tool.invoke(&call(
        "hall/note.md",
        "new",
        "",
        "# note
",
    ))
    .unwrap();
    assert!(tmp.path().join("hall/note.md").is_file());
    let refused = tool
        .invoke(&call("hall/lex.rs", "new", "", "fn main() {}"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::OutsideWriteDomain);
    assert!(
        refused.subject().contains("hall/lex.rs"),
        "the refusal names the file, not the room"
    );
    let plan = tool
        .invoke(&call("hall/Roadmap.md", "new", "", "| 0 |"))
        .unwrap_err();
    assert_eq!(*plan.code(), AxCode::OutsideWriteDomain);
}
