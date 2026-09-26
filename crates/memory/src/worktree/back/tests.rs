// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::error::MemoryError;
use kernel::GitOid;
use std::path::Path;

/// Commits `body` as `notes.md` on the trunk and returns the point.
fn commit(repo: &git2::Repository, body: &str) -> GitOid {
    let dir = repo.workdir().unwrap().to_path_buf();
    std::fs::write(dir.join("notes.md"), body).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("notes.md")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("city", "city@example.invalid").unwrap();
    let parent = repo.head().ok().map(|h| h.peel_to_commit().unwrap());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, body, &tree, &parents)
        .unwrap();
    GitOid::parse(&oid.to_string()).unwrap()
}

/// A city with two points on its trunk: "first", then "second".
fn city(dir: &Path) -> (Worktrees, GitOid, GitOid) {
    let repo = git2::Repository::init(dir).unwrap();
    // Checked-out bytes equal to the committed ones on every host.
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    let first = commit(&repo, "first\n");
    let second = commit(&repo, "second\n");
    (Worktrees::open(dir).unwrap(), first, second)
}

fn name(raw: &str) -> WorktreeName {
    WorktreeName::parse(raw).unwrap()
}

fn read(root: &Path) -> String {
    std::fs::read_to_string(root.join("notes.md")).unwrap()
}

fn head(dir: &Path) -> String {
    let repo = git2::Repository::open(dir).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    head.id().to_string()
}

#[test]
fn going_back_opens_a_tree_at_the_point_and_leaves_the_trunk_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let (trees, first, second) = city(dir.path());

    let back = trees.claim_at(&name("back-1"), &first).unwrap();

    assert_eq!(
        (read(back.path()), read(dir.path()), head(dir.path())),
        (
            "first\n".to_owned(),
            "second\n".to_owned(),
            second.to_string()
        )
    );
}

#[test]
fn going_back_refuses_a_name_that_holds_a_line_of_work() {
    let dir = tempfile::tempdir().unwrap();
    let (trees, first, _) = city(dir.path());
    let held = trees.claim(&name("node-1")).unwrap();
    trees.release(held).unwrap();

    let refused = trees.claim_at(&name("node-1"), &first);

    assert!(
        matches!(refused, Err(MemoryError::WorktreeBusy { .. })),
        "the released node's branch is still its line of work: {refused:?}"
    );
}

#[test]
fn restoring_a_file_takes_it_back_from_the_point_into_that_tree_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (trees, first, _) = city(dir.path());
    let mine = trees.claim(&name("node-1")).unwrap();
    let theirs = trees.claim(&name("node-2")).unwrap();
    std::fs::write(mine.path().join("later.md"), "not at the point\n").unwrap();

    trees
        .restore_file(&mine, &first, Path::new("notes.md"))
        .unwrap();
    trees
        .restore_file(&mine, &first, Path::new("later.md"))
        .unwrap();

    assert_eq!(
        (
            read(mine.path()),
            mine.path().join("later.md").exists(),
            read(theirs.path()),
            read(dir.path())
        ),
        (
            "first\n".to_owned(),
            false,
            "second\n".to_owned(),
            "second\n".to_owned()
        )
    );
}

#[test]
fn restoring_refuses_a_path_that_climbs_out_of_the_tree() {
    let dir = tempfile::tempdir().unwrap();
    let (trees, first, _) = city(dir.path());
    let mine = trees.claim(&name("node-1")).unwrap();

    let refused = trees.restore_file(&mine, &first, Path::new("../notes.md"));

    assert!(
        matches!(
            refused,
            Err(MemoryError::Worktree {
                op: "restore a file from a point",
                ..
            })
        ),
        "{refused:?}"
    );
}

#[test]
fn restoring_through_a_hard_link_leaves_the_linked_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let (trees, first, _) = city(dir.path());
    let mine = trees.claim(&name("node-1")).unwrap();
    let theirs = trees.claim(&name("node-2")).unwrap();
    std::fs::remove_file(mine.path().join("notes.md")).unwrap();
    std::fs::hard_link(theirs.path().join("notes.md"), mine.path().join("notes.md")).unwrap();

    let restored = trees.restore_file(&mine, &first, Path::new("notes.md"));

    let theirs_now = read(theirs.path());
    if restored.is_ok() {
        assert_eq!(
            (read(mine.path()), theirs_now),
            (
                "first
"
                .to_owned(),
                "second
"
                .to_owned()
            )
        );
    } else {
        assert_eq!(
            theirs_now,
            "second
"
            .to_owned(),
            "{restored:?}"
        );
    }
}
