// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn an_edit_after_a_read_lands_echoes_a_diff_and_admits_the_next_without_a_read() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "one\ntwo\nthree\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let outcome = tool.invoke(&call("work/a.txt", "two", "TWO")).unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    let diff = result["diff"].as_str().unwrap();
    assert!(diff.contains("-two"), "{diff}");
    assert!(diff.contains("+TWO"), "{diff}");
    assert_eq!(result.get("new_version"), None);
    // The run's own write is what it now knows the file to be.
    tool.invoke(&call("work/a.txt", "three", "THREE")).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "one\nTWO\nTHREE\n"
    );
}

#[test]
fn an_edit_by_a_run_that_never_read_the_file_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "current\n").unwrap();
    let err = tool
        .invoke(&call("work/a.txt", "current", "next"))
        .unwrap_err();
    assert_eq!(
        (
            *err.code(),
            err.recovery().contains("read work/a.txt first"),
            std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        ),
        (AxCode::InvalidArgs, true, "current\n".to_owned())
    );
}

/// The strict side of D106: a file another writer changed after this
/// run's read is refused even where `old` still matches once, and the
/// refusal carries the passage as it now reads, which counts as a look.
#[test]
fn a_file_changed_after_the_read_is_refused_with_the_passage_as_it_now_reads() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let path = tmp.path().join("work/a.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let staged = tmp.path().join("work/a.txt.other");
    std::fs::write(&staged, "alpha\nbeta, said another\ngamma\n").unwrap();
    std::fs::rename(&staged, &path).unwrap();

    let err = tool
        .invoke(&call("work/a.txt", "beta", "BETA"))
        .unwrap_err();
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(
        err.subject()
            .contains("lines 1-3 as the file now reads:\nalpha\nbeta, said another\ngamma\n"),
        "{}",
        err.subject()
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "alpha\nbeta, said another\ngamma\n"
    );
    tool.invoke(&call("work/a.txt", "beta, said another", "BETA"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "alpha\nBETA\ngamma\n"
    );
}

/// Two runs that read one file: the first edit lands, the second is
/// refused with the passage as the first left it, and its retry lands
/// on top - neither change is lost (§8-64-2, `two_runs_on_one_look_land_once`).
#[test]
fn two_runs_editing_one_file_lose_neither_change() {
    let tmp = tempfile::tempdir().unwrap();
    let locks = PathLocks::default();
    let first = tool_sharing(tmp.path(), locks.clone());
    let second = tool_sharing(tmp.path(), locks);
    std::fs::write(tmp.path().join("work/a.txt"), "head\ntail\n").unwrap();
    look(&first, tmp.path(), "work/a.txt");
    look(&second, tmp.path(), "work/a.txt");

    first.invoke(&call("work/a.txt", "head", "HEAD")).unwrap();
    let refused = second
        .invoke(&call("work/a.txt", "tail", "TAIL"))
        .unwrap_err();
    second.invoke(&call("work/a.txt", "tail", "TAIL")).unwrap();

    assert_eq!(
        (
            *refused.code(),
            std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        ),
        (AxCode::VersionConflict, "HEAD\nTAIL\n".to_owned())
    );
}

#[test]
fn old_empty_creates_the_file_and_parents() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let outcome = tool
        .invoke(&call("work/room/notes.md", "", "first line\n"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/room/notes.md")).unwrap(),
        "first line\n"
    );
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert!(result["diff"].as_str().unwrap().contains("+first line"));
}

#[test]
fn creating_over_an_existing_file_is_refused_and_leaves_it_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "already\n").unwrap();
    let err = tool.invoke(&call("work/a.txt", "", "other\n")).unwrap_err();
    assert_eq!(*err.code(), AxCode::VersionConflict);
    assert!(err.subject().contains("already exists"), "{err}");
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        "already\n"
    );
}

#[test]
fn a_path_outside_the_write_domain_is_refused_before_the_disk_is_touched() {
    let tmp = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let beyond = elsewhere.path().join("abs.txt");
    std::fs::write(tmp.path().join("outside.txt"), "x").unwrap();
    let tool = tool(tmp.path());
    for hostile in ["outside.txt", "elsewhere/f.md", ".sprawling/ledger/x"] {
        let err = match tool.invoke(&call(hostile, "", "y")) {
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
    for illegal in ["../evil.txt", "work//x"] {
        let err = match tool.invoke(&call(illegal, "", "y")) {
            Err(err) => err,
            Ok(_) => panic!("{illegal} must be refused"),
        };
        assert_eq!(*err.code(), AxCode::InvalidArgs, "{illegal}");
    }
    // A path this platform calls absolute is an address only inside the
    // city (runtime D4): outside it the refusal points at `exec`,
    // and the create form leaves nothing behind there.
    let err = match tool.invoke(&call(beyond.to_str().unwrap(), "", "y")) {
        Err(err) => err,
        Ok(_) => panic!("{} must be refused", beyond.display()),
    };
    assert_eq!(
        (
            err.code(),
            err.recovery().contains("`exec`"),
            beyond.exists()
        ),
        (&AxCode::GateDenied, true, false),
        "{err:?}"
    );
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("outside.txt")).unwrap(),
        "x"
    );
}

#[test]
fn a_missing_file_names_the_create_form_in_its_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let err = tool.invoke(&call("work/absent.md", "a", "b")).unwrap_err();
    assert!(err.recovery().contains("old_string:\"\""), "{err}");
}

#[test]
fn zero_or_many_matches_refuse_with_the_count() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "x\nx\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let err = tool.invoke(&call("work/a.txt", "x", "y")).unwrap_err();
    assert_eq!(*err.code(), AxCode::InvalidArgs);
    assert!(err.subject().contains("2 times"), "{}", err.subject());

    let err = tool.invoke(&call("work/a.txt", "absent", "y")).unwrap_err();
    assert!(err.subject().contains("0 times"), "{}", err.subject());
}

#[test]
fn many_matches_name_both_ways_on_in_the_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "x\nx\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let err = tool.invoke(&call("work/a.txt", "x", "y")).unwrap_err();
    assert!(
        err.recovery().contains("old_string") && err.recovery().contains("replace_all: true"),
        "{err}"
    );
}

#[test]
fn replace_all_changes_every_occurrence_and_still_refuses_none() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "x\nkeep\nx\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let every = |old: &str| {
        edit_call(serde_json::json!({
            "file_path": "work/a.txt", "old_string": old, "new_string": "y", "replace_all": true,
        }))
    };
    tool.invoke(&every("x")).unwrap();
    let err = tool.invoke(&every("absent")).unwrap_err();
    assert_eq!(
        (
            *err.code(),
            err.subject().contains("0 times"),
            std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        ),
        (AxCode::InvalidArgs, true, "y\nkeep\ny\n".to_owned())
    );
}

#[test]
fn replace_all_on_a_create_is_refused_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let err = tool
        .invoke(&edit_call(serde_json::json!({
            "file_path": "work/new.md", "old_string": "", "new_string": "body", "replace_all": true,
        })))
        .unwrap_err();
    assert_eq!(
        (*err.code(), tmp.path().join("work/new.md").exists()),
        (AxCode::InvalidArgs, false)
    );
}

/// D115: a call spelled the way an older build's history spells it is
/// refused by the names this build takes, not read the old way.
#[test]
fn an_edit_spelled_the_older_way_is_refused_with_the_names_it_takes() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::write(tmp.path().join("work/a.txt"), "one\n").unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    let err = tool
        .invoke(&edit_call(serde_json::json!({
            "path": "work/a.txt", "old": "one", "new": "two",
        })))
        .unwrap_err();
    assert_eq!(
        (
            *err.code(),
            err.recovery()
                .contains("file_path, old_string and new_string"),
            std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        ),
        (AxCode::InvalidArgs, true, "one\n".to_owned())
    );
}

#[test]
fn a_call_for_another_tool_is_refused_not_routed() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let mut wrong = call("work/a.txt", "a", "b");
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
    let tool = EditTool::new(
        tmp.path(),
        Address::parse("hall/mayor").unwrap(),
        domain,
        desk(PathLocks::default()),
    )
    .unwrap();
    tool.invoke(&call("hall/note.md", "", "# note\n")).unwrap();
    assert!(tmp.path().join("hall/note.md").is_file());
    let refused = tool
        .invoke(&call("hall/lex.rs", "", "fn main() {}"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::OutsideWriteDomain);
    assert!(
        refused.subject().contains("hall/lex.rs"),
        "the refusal names the file, not the room"
    );
    let plan = tool
        .invoke(&call("hall/Roadmap.md", "", "| 0 |"))
        .unwrap_err();
    assert_eq!(*plan.code(), AxCode::OutsideWriteDomain);
}
