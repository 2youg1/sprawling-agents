// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

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
