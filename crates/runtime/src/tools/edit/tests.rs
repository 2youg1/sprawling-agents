// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::PolicyCell;
use kernel::{Address, Mode, RunPolicy};

fn tool(root: &Path) -> EditTool {
    tool_sharing(root, PathLocks::default())
}

/// A run's edit tool with a table of its own, holding `locks` - the
/// city's one table - so two of these are two runs in one city.
fn tool_sharing(root: &Path, locks: PathLocks) -> EditTool {
    let work = Address::parse("work").unwrap();
    let domain = kernel::WriteDomain::new(vec![work.clone()]).unwrap();
    std::fs::create_dir_all(root.join("work")).unwrap();
    EditTool::new(root, work, domain, desk(locks)).unwrap()
}

fn desk(locks: PathLocks) -> EditDesk {
    EditDesk {
        policy: PolicyCell::new(RunPolicy::of(Mode::Work)).reader(),
        seen: ReadStamps::default(),
        locks,
    }
}

/// What a whole read of `rel` records: the stamp `read` takes off its
/// handle (`tools::read::tests::looks` drives the read itself).
fn look(tool: &EditTool, root: &Path, rel: &str) {
    let stamp = Stamp::of(&root.join(rel)).unwrap();
    tool.desk
        .seen
        .look(&Address::parse(rel).unwrap(), stamp.clone(), stamp);
}

fn call(path: &str, old: &str, new: &str) -> ToolCall {
    let mut args = Map::new();
    for (k, v) in [("path", path), ("old", old), ("new", new)] {
        args.insert(k.to_owned(), Value::String(v.to_owned()));
    }
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

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
            .contains("lines 0-2 as the file now reads:\nalpha\nbeta, said another\ngamma\n"),
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
    assert!(err.recovery().contains("old:\"\""), "{err}");
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

/// Places a junction at `to` leading to `from` (a symlink off
/// Windows). `false` when this machine hands out no link at all.
#[cfg(windows)]
fn place_link(_file: bool, from: &Path, to: &Path) -> bool {
    std::process::Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            &to.display().to_string(),
            &from.display().to_string(),
        ])
        .output()
        .is_ok_and(|out| out.status.success())
}

#[cfg(not(windows))]
fn place_link(_file: bool, from: &Path, to: &Path) -> bool {
    std::os::unix::fs::symlink(from, to).is_ok()
}

/// The acceptance of the protected-metadata rule at a run's own write
/// face: `work/alias-dir/pre-run` crossing a junction into
/// `.git/hooks` is privilege escalation through a name the write-domain
/// gate already allowed. The write is refused whole, and the hook keeps
/// its bytes.
#[test]
fn an_edit_through_a_link_to_a_reserved_path_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::create_dir_all(tmp.path().join(".git").join("hooks")).unwrap();
    let hook = tmp.path().join(".git").join("hooks").join("pre-run");
    std::fs::write(&hook, b"hook-body").unwrap();
    if !place_link(
        true,
        &tmp.path().join(".git").join("hooks"),
        &tmp.path().join("work").join("alias-dir"),
    ) {
        return;
    }

    let refused = tool
        .invoke(&call("work/alias-dir/pre-run", "hook-body", "escalated"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::OutsideWriteDomain);
    assert_eq!(
        std::fs::read(&hook).unwrap(),
        b"hook-body",
        "the reserved file the link reaches is byte-identical"
    );
}

/// Creating below a name that is a link must not make a single
/// directory at the link's target. The alias check has to run before
/// `create_dir_all`, or a protected subtree gains a directory through a
/// name the write-domain gate already allowed (`crates/storage/Spec.lean` §8-25).
#[test]
fn creating_below_a_link_leaves_the_link_target_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let hooks = tmp.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    if !place_link(true, &hooks, &tmp.path().join("work").join("alias-dir")) {
        return;
    }

    let refused = tool
        .invoke(&call("work/alias-dir/sub/x.md", "", "body"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::OutsideWriteDomain);
    assert_eq!(
        std::fs::read_dir(&hooks).unwrap().count(),
        0,
        "no directory is made through the link"
    );
}

/// The hard-link arm: whatever verdict an edit whose name shares an
/// inode with a reserved file is greeted with, the reserved file is
/// whole afterwards - the write never lands in it.
#[test]
fn an_edit_through_a_hard_link_leaves_the_reserved_file_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    std::fs::create_dir_all(tmp.path().join(".git").join("hooks")).unwrap();
    let hook = tmp.path().join(".git").join("hooks").join("pre-run");
    std::fs::write(&hook, b"hook-body").unwrap();
    if !std::fs::hard_link(&hook, tmp.path().join("work").join("a.txt")).is_ok() {
        return;
    }

    look(&tool, tmp.path(), "work/a.txt");
    let outcome = tool.invoke(&call("work/a.txt", "hook-body", "escalated"));
    if outcome.is_ok() {
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("work").join("a.txt")).unwrap(),
            "escalated",
            "the write lands at the name it was given"
        );
    }
    assert_eq!(
        std::fs::read(&hook).unwrap(),
        b"hook-body",
        "the reserved file behind the other name is byte-identical: the write never landed"
    );
}

/// Where a person keeps a city is their placement, not a write a run
/// could redirect: a city reached through a link above its root - macOS
/// puts every temporary directory under `/var`, which is a link to
/// `/private/var` - is edited like any other (`crates/storage/Spec.lean` §8-12).
#[test]
fn a_city_placed_below_a_link_is_edited_like_any_other() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("real")).unwrap();
    if !place_link(true, &tmp.path().join("real"), &tmp.path().join("placed")) {
        return;
    }
    let city = tmp.path().join("placed");
    let tool = tool(&city);
    std::fs::write(city.join("work/a.txt"), "draft\n").unwrap();

    look(&tool, &city, "work/a.txt");
    let edited = tool.invoke(&call("work/a.txt", "draft", "final"));
    let created = tool.invoke(&call("work/b.txt", "", "made\n"));

    assert_eq!(
        (
            edited.map(|_| ()),
            created.map(|_| ()),
            std::fs::read_to_string(tmp.path().join("real/work/a.txt")).unwrap(),
            std::fs::read_to_string(tmp.path().join("real/work/b.txt")).unwrap(),
        ),
        (Ok(()), Ok(()), "final\n".to_owned(), "made\n".to_owned())
    );
}

/// A passage copied out of a `read` answer with its line marks lands on
/// the file's own bytes, and the marks copied into the replacement are
/// not written into the file.
#[test]
fn a_passage_copied_with_its_line_marks_edits_the_lines_it_marks() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..12).map(|n| format!("row {n}\n")).collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    tool.invoke(&call(
        "work/a.txt",
        "row 9\n10|row 10\n",
        "row 9\n10|row ten\n",
    ))
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 10\n", "row ten\n")
    );
}

/// A file whose own lines look like marks is matched as the passage came:
/// an exact match is never second-guessed.
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

/// A numbered line the model means to write stays in the file: only the
/// marks copied from the passage it replaces come off the replacement.
#[test]
fn a_numbered_line_added_in_the_replacement_is_written_as_it_came() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..12).map(|n| format!("row {n}\n")).collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    tool.invoke(&call("work/a.txt", "10|row 10\n", "10|row ten\n12|keep\n"))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 10\n", "row ten\n12|keep\n")
    );
}

/// A passage copied with its marks over a line the file itself starts
/// with digits and a separator edits that line: the file's own text is
/// not mistaken for a mark, because no read draws a mark there.
#[test]
fn a_copied_passage_keeps_a_numbered_line_the_file_holds() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = tool(tmp.path());
    let before: String = (0..13)
        .map(|n| match n {
            11 => "7|literal\n".to_owned(),
            _ => format!("row {n}\n"),
        })
        .collect();
    std::fs::write(tmp.path().join("work/a.txt"), &before).unwrap();
    look(&tool, tmp.path(), "work/a.txt");
    tool.invoke(&call(
        "work/a.txt",
        "10|row 10\n7|literal\n",
        "10|row 10\n7|changed\n",
    ))
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("7|literal\n", "7|changed\n")
    );
}

/// A passage copied out of a `search` hit drawn with context lands on the
/// file's own bytes: a hit block is marked in the grammar a read answer
/// starting on the block's first line is, so `edit` sees through it.
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
    assert_eq!(old, "5|row 5\nrow 6\nrow 7\n");
    let new = old.replace("row 7", "row seven");
    tool.invoke(&call("work/a.txt", &old, &new)).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("work/a.txt")).unwrap(),
        before.replace("row 7\n", "row seven\n")
    );
}
