// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::checkpoint::{Checkpoint, ModelChoice, Provenance};
use kernel::{Address, B3Hash, Effort, RunId, TimeMs};
use std::path::Path;

pub(crate) fn owner() -> Provenance {
    Provenance::new(
        RunId::CITY,
        Address::parse("lab/owner").unwrap(),
        B3Hash::digest(b"a city"),
        ModelChoice {
            id: "test-model".to_owned(),
            effort: Some(Effort::Low),
        },
    )
}

fn landing(t: u64, of: &Provenance, reviewed_by_person: bool) -> Landing<'_> {
    Landing {
        t: TimeMs::new(t),
        of,
        subject: "merge: node-1",
        reviewed_by_person,
    }
}

fn city(dir: &Path) -> Worktrees {
    std::fs::create_dir_all(dir.join("lab")).unwrap();
    std::fs::write(dir.join("lab").join("notes.md"), b"first\n").unwrap();
    let mut checkpoint = Checkpoint::open(dir).unwrap();
    checkpoint
        .ensure_base("lab", TimeMs::new(1_000), &owner())
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
        .land(TimeMs::new(2_000), &owner(), "checkpoint: lab")
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
        .land(TimeMs::new(2_000), &owner(), "checkpoint: lab")
        .unwrap();

    trees
        .plan_merge(lease.name())
        .unwrap()
        .apply(&landing(5_000, &owner(), false))
        .unwrap();
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
        .land(TimeMs::new(3_000), &owner(), "checkpoint: lab")
        .unwrap();
    std::fs::write(stale.path().join("lab").join("notes.md"), b"stale work\n").unwrap();
    Checkpoint::open(stale.path())
        .unwrap()
        .land(TimeMs::new(4_000), &owner(), "checkpoint: lab")
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

/// A merge is a thing that happened, so it has a commit of its own: two
/// parents, and the trailers of the run that did the merging.
#[test]
fn a_merge_lands_as_a_two_parent_commit_carrying_the_merging_runs_trailers() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1")).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"from node\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(TimeMs::new(2_000), &owner(), "checkpoint: lab")
        .unwrap();

    trees
        .plan_merge(lease.name())
        .unwrap()
        .apply(&landing(5_000, &owner(), false))
        .unwrap();

    let repo = git2::Repository::open(dir.path()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.parent_count(), 2, "a merge has two parents");
    let message = head.message().unwrap();
    assert!(message.starts_with("merge: node-1\n\n"), "{message}");
    for trailer in [
        "Sprawling-Run: ",
        "Sprawling-Actor: lab/owner",
        "Sprawling-Model: test-model",
        "Sprawling-Effort: low",
        "Sprawling-City: ",
    ] {
        assert!(
            message.contains(trailer),
            "{trailer} missing from {message}"
        );
    }
    assert!(
        !message.contains("Reviewed-by:"),
        "nobody claimed to have looked: {message}"
    );
}

/// A person's name is the person's. The city writes it only when the
/// caller says a person looked and this machine's git config says who.
#[test]
fn a_person_who_looked_is_named_from_the_repositorys_own_config() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    {
        let repo = git2::Repository::open(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Ada Lovelace").unwrap();
        config.set_str("user.email", "ada@example.org").unwrap();
    }
    let lease = trees.claim(&name("node-1")).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"from node\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(TimeMs::new(2_000), &owner(), "checkpoint: lab")
        .unwrap();
    trees
        .plan_merge(lease.name())
        .unwrap()
        .apply(&landing(5_000, &owner(), true))
        .unwrap();

    let repo = git2::Repository::open(dir.path()).unwrap();
    let message = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .message()
        .unwrap()
        .to_owned();
    assert!(
        message.contains("Reviewed-by: Ada Lovelace <ada@example.org>\n"),
        "{message}"
    );
}
