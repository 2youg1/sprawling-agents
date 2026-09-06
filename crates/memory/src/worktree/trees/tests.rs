// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::checkpoint::Checkpoint;
use kernel::TimeMs;
use std::path::Path;
fn city(dir: &Path) -> Worktrees {
    std::fs::create_dir_all(dir.join("lab")).unwrap();
    std::fs::write(dir.join("lab").join("notes.md"), b"first\n").unwrap();
    let mut checkpoint = Checkpoint::open(dir).unwrap();
    checkpoint
        .wave_pre("lab", TimeMs::new(1_000), "owner")
        .unwrap();
    Worktrees::open(dir).unwrap()
}
fn name(raw: &str) -> WorktreeName {
    WorktreeName::parse(raw).unwrap()
}

#[test]
fn two_nodes_get_two_trees_and_neither_sees_the_others_work() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());

    let first = trees.claim(&name("node-1")).unwrap();
    let second = trees.claim(&name("node-2")).unwrap();
    assert_ne!(first.path(), second.path());

    std::fs::write(first.path().join("lab").join("notes.md"), b"mine\n").unwrap();
    let other = std::fs::read_to_string(second.path().join("lab").join("notes.md")).unwrap();
    assert_eq!(other, "first\n", "the other node still sees the checkpoint");
    let city_copy = std::fs::read_to_string(dir.path().join("lab").join("notes.md")).unwrap();
    assert_eq!(city_copy, "first\n", "and so does the city");
}

#[test]
fn one_node_holds_one_tree_and_the_second_claim_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let held = trees.claim(&name("node-1")).unwrap();

    let err = trees.claim(&name("node-1")).unwrap_err();
    let ax = err.into_ax();
    assert_eq!(ax.code(), &kernel::AxCode::WorktreeBusy);
    assert!(ax.subject().contains("node-1"));
    assert!(ax.recovery().contains("release"));

    trees.release(held).unwrap();
    assert!(
        trees.live().unwrap().is_empty(),
        "a released tree is gone from the repository, not just from disk"
    );
    trees.claim(&name("node-1")).unwrap();
}

#[test]
fn a_released_tree_takes_its_files_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1")).unwrap();
    let path = lease.path().to_path_buf();
    assert!(path.join("lab").join("notes.md").exists());

    trees.release(lease).unwrap();
    assert!(!path.exists());
    assert!(dir.path().join("lab").join("notes.md").exists());
}

#[test]
fn a_node_that_comes_back_finds_what_it_committed_and_not_what_it_did_not() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1")).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"committed\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .wave_pre("lab", TimeMs::new(2_000), "node-1")
        .unwrap();
    std::fs::write(lease.path().join("lab").join("draft.md"), b"uncommitted\n").unwrap();
    trees.release(lease).unwrap();

    let again = trees.claim(&name("node-1")).unwrap();
    assert_eq!(
        std::fs::read_to_string(again.path().join("lab").join("notes.md")).unwrap(),
        "committed\n",
        "the node's branch outlives its tree"
    );
    assert!(
        !again.path().join("lab").join("draft.md").exists(),
        "what was never committed was never the node's work"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab").join("notes.md")).unwrap(),
        "first\n",
        "and none of it reached the city, which is what the PR flow is for"
    );
}

#[test]
fn a_city_with_no_checkpoint_is_told_why_it_cannot_have_a_tree() {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let trees = Worktrees::open(dir.path()).unwrap();

    let err = trees.claim(&name("node-1")).unwrap_err().into_ax();
    assert!(err.subject().contains("no checkpoint"));
}

#[test]
fn a_verified_node_lands_in_the_city_and_a_stale_one_is_sent_back() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1")).unwrap();
    std::fs::write(
        lease.path().join("lab").join("notes.md"),
        b"from the node\n",
    )
    .unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .wave_pre("lab", TimeMs::new(2_000), "node-1")
        .unwrap();

    trees.plan_merge(lease.name()).unwrap().apply().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab").join("notes.md")).unwrap(),
        "from the node\n",
        "the city works on what the node produced"
    );

    // A second node that branched before the merge is now behind.
    let stale = trees.claim(&name("node-2")).unwrap();
    std::fs::write(dir.path().join("lab").join("other.md"), b"trunk moved\n").unwrap();
    Checkpoint::open(dir.path())
        .unwrap()
        .wave_pre("lab", TimeMs::new(3_000), "owner")
        .unwrap();
    std::fs::write(stale.path().join("lab").join("notes.md"), b"stale work\n").unwrap();
    Checkpoint::open(stale.path())
        .unwrap()
        .wave_pre("lab", TimeMs::new(4_000), "node-2")
        .unwrap();

    let err = trees.plan_merge(stale.name()).unwrap_err().into_ax();
    assert_eq!(err.code(), &kernel::AxCode::VersionConflict);
    assert!(err.recovery().contains("verified again"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab").join("notes.md")).unwrap(),
        "from the node\n",
        "a refused merge changes nothing in the city"
    );
}

#[test]
fn a_city_with_no_repository_is_refused_rather_than_given_one() {
    let dir = tempfile::tempdir().unwrap();
    let err = Worktrees::open(dir.path()).unwrap_err().into_ax();
    assert_eq!(err.code(), &kernel::AxCode::StorageFatal);
}
