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

pub(crate) fn city(dir: &Path) -> Worktrees {
    std::fs::create_dir_all(dir.join("lab")).unwrap();
    std::fs::write(dir.join("lab").join("notes.md"), b"first\n").unwrap();
    let mut checkpoint = Checkpoint::open(dir).unwrap();
    checkpoint
        .ensure_base(&["lab".to_owned()], TimeMs::new(1_000), &owner())
        .unwrap();
    Worktrees::open(dir).unwrap()
}
pub(super) fn name(raw: &str) -> WorktreeName {
    WorktreeName::parse(raw).unwrap()
}

pub(crate) fn bulk_city(dir: &Path, files: u64) -> Worktrees {
    let bulk = dir.join("bulk");
    std::fs::create_dir_all(&bulk).unwrap();
    for i in 0..files {
        std::fs::write(bulk.join(format!("file-{i:04}.txt")), b"sixteen bytes ..").unwrap();
    }
    Checkpoint::open(dir)
        .unwrap()
        .ensure_base(&["bulk".to_owned()], TimeMs::new(1_000), &owner())
        .unwrap();
    Worktrees::open(dir).unwrap()
}

pub(crate) fn entries(root: &Path) -> u64 {
    let count = |dir: &Path| u64::try_from(std::fs::read_dir(dir).unwrap().count()).unwrap();
    count(root) + count(&root.join("bulk"))
}

#[test]
fn two_nodes_get_two_trees_and_neither_sees_the_others_work() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());

    let first = trees.claim(&name("node-1"), &[]).unwrap();
    let second = trees.claim(&name("node-2"), &[]).unwrap();
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
    let held = trees.claim(&name("node-1"), &[]).unwrap();

    let err = trees.claim(&name("node-1"), &[]).unwrap_err();
    let ax = err.into_ax();
    assert_eq!(ax.code(), &kernel::AxCode::WorktreeBusy);
    assert!(ax.subject().contains("node-1"));
    assert!(ax.recovery().contains("release"));

    trees.release(held).unwrap();
    trees.claim(&name("node-1"), &[]).unwrap();
}

#[test]
fn the_city_s_own_bookkeeping_does_not_count_against_the_ceiling() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let ledger = dir.path().join(kernel::RESERVED_PREFIX).join("ledger");
    std::fs::create_dir_all(&ledger).unwrap();
    let bulk = vec![b'x'; 4096];
    std::fs::write(ledger.join("ledger-0.jsonl"), &bulk).unwrap();

    let source = trees.repo.workdir().unwrap().to_path_buf();
    let measured = crate::worktree::weight::measure(&source).unwrap().bytes;
    assert!(
        measured.get() < 4096,
        "the ledger is not somebody's working tree: {} bytes",
        measured.get()
    );
    trees.claim(&name("node-1"), &[]).unwrap();
}

#[test]
fn a_registered_tree_with_no_directory_is_taken_back_rather_than_locked_away() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    let path = lease.path().to_path_buf();
    std::fs::remove_dir_all(&path).unwrap();
    assert_eq!(trees.live().unwrap().len(), 1, "git still lists it");

    let again = trees.claim(&name("node-1"), &[]).unwrap();
    assert!(again.path().join("lab").join("notes.md").exists());
}

#[test]
fn a_node_that_comes_back_finds_what_it_committed_and_not_what_it_did_not() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"committed\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
        .unwrap();
    std::fs::write(lease.path().join("lab").join("draft.md"), b"uncommitted\n").unwrap();
    trees.release(lease).unwrap();

    let again = trees.claim(&name("node-1"), &[]).unwrap();
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

    let err = trees.claim(&name("node-1"), &[]).unwrap_err().into_ax();
    assert!(err.subject().contains("no checkpoint"));
}

#[test]
fn a_landing_plan_cannot_replace_a_trunk_that_advanced_after_review() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let node = trees.claim(&name("node-1"), &[]).unwrap();
    let plan = trees.plan_merge(node.name()).unwrap();
    let other = dir.path().join("lab").join("other.md");
    std::fs::write(&other, b"independent work").unwrap();
    let landed = Checkpoint::open(dir.path())
        .unwrap()
        .land(&[], TimeMs::new(3_000), &owner(), "independent change")
        .unwrap();
    let rejected = plan.apply(&landing(4_000, &owner(), false)).unwrap_err();
    assert_eq!(rejected.into_ax().code(), &kernel::AxCode::VersionConflict);
    assert_eq!(std::fs::read(other).unwrap(), b"independent work");
    assert_eq!(
        trees.repo.head().unwrap().target().unwrap().to_string(),
        landed.oid.to_string()
    );
}

#[test]
fn a_verified_node_lands_in_the_city_and_a_stale_one_is_sent_back() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    std::fs::write(
        lease.path().join("lab").join("notes.md"),
        b"from the node\n",
    )
    .unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
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

    let stale = trees.claim(&name("node-2"), &[]).unwrap();
    std::fs::write(dir.path().join("lab").join("other.md"), b"trunk moved\n").unwrap();
    Checkpoint::open(dir.path())
        .unwrap()
        .land(&[], TimeMs::new(3_000), &owner(), "checkpoint: lab")
        .unwrap();
    std::fs::write(stale.path().join("lab").join("notes.md"), b"stale work\n").unwrap();
    Checkpoint::open(stale.path())
        .unwrap()
        .land(&[], TimeMs::new(4_000), &owner(), "checkpoint: lab")
        .unwrap();

    let candidate = trees
        .repo
        .find_branch(stale.name().as_str(), git2::BranchType::Local)
        .unwrap()
        .get()
        .target()
        .unwrap();
    let point = kernel::GitOid::parse(&candidate.to_string()).unwrap();
    let err = trees
        .plan_reviewed(stale.name(), &point)
        .unwrap_err()
        .into_ax();
    assert_eq!(err.code(), &kernel::AxCode::VersionConflict);
    let crate::ReviewPlan::Renewed(renewed) = trees.prepare_review(stale.name(), &point).unwrap()
    else {
        panic!("an advanced trunk needs a fresh reviewed snapshot");
    };
    assert_ne!(point, renewed);
    let crate::ReviewPlan::Ready(plan) = trees.prepare_review(stale.name(), &renewed).unwrap()
    else {
        panic!("the refreshed candidate includes this trunk");
    };
    assert_eq!(plan.commit(), renewed.to_string());
    let snapshot = trees.commit_at(&renewed).unwrap().tree().unwrap();
    for path in ["lab/notes.md", "lab/other.md"] {
        assert!(snapshot.get_path(Path::new(path)).is_ok());
    }
    trees
        .plan_completed(stale.name(), &point)
        .unwrap()
        .apply(&landing(6_000, &owner(), false))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab/notes.md")).unwrap(),
        "stale work\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab/other.md")).unwrap(),
        "trunk moved\n"
    );
}

#[test]
fn a_city_with_no_repository_is_refused_rather_than_given_one() {
    let dir = tempfile::tempdir().unwrap();
    let err = Worktrees::open(dir.path()).unwrap_err().into_ax();
    assert_eq!(err.code(), &kernel::AxCode::StorageFatal);
}

#[test]
fn a_merge_lands_as_a_two_parent_commit_carrying_the_merging_runs_trailers() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"from node\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
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
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    std::fs::write(lease.path().join("lab").join("notes.md"), b"from node\n").unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
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

#[test]
fn a_tree_is_never_placed_through_a_link() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let home = dir.path().join(".sprawling").join("worktrees");
    std::fs::create_dir_all(&home).unwrap();
    let kept = dir.path().join(".sprawling").join("kept");
    std::fs::create_dir_all(&kept).unwrap();
    if !crate::alias::tests::place_link(true, &kept, &home.join("node-1")) {
        return;
    }
    let err = trees.claim(&name("node-1"), &[]).unwrap_err();
    assert_eq!(err.into_ax().code(), &kernel::AxCode::OutsideWriteDomain);
    assert_eq!(
        std::fs::read_dir(&kept).unwrap().count(),
        0,
        "the reserved directory the link reaches holds nothing new"
    );
}

#[test]
fn a_merge_keeps_the_persons_uncommitted_edit_and_names_it_in_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    std::fs::write(
        lease.path().join("lab").join("notes.md"),
        b"from the node\n",
    )
    .unwrap();
    Checkpoint::open(lease.path())
        .unwrap()
        .land(&[], TimeMs::new(2_000), &owner(), "checkpoint: lab")
        .unwrap();
    std::fs::write(
        dir.path().join("lab").join("notes.md"),
        b"the person's edit\n",
    )
    .unwrap();

    let refused = trees
        .plan_merge(lease.name())
        .and_then(|plan| plan.apply(&landing(5_000, &owner(), false)));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab").join("notes.md")).unwrap(),
        "the person's edit\n",
        "a merge never discards an edit the person has not committed"
    );
    let err = refused.unwrap_err().into_ax();
    assert_eq!(err.code(), &kernel::AxCode::VersionConflict);
    assert!(err.to_string().contains("lab/notes.md"), "{err}");
}

#[test]
fn the_citys_git_reads_its_repository_config_and_no_file_of_the_users() {
    let dir = tempfile::tempdir().unwrap();
    let trees = city(dir.path());
    let lease = trees.claim(&name("node-1"), &[]).unwrap();
    let tree = trees.repo.find_worktree(lease.name().as_str()).unwrap();
    let handles = [
        trees.repo.config().unwrap(),
        super::open_tree(&tree).unwrap().config().unwrap(),
    ];
    let levels: std::collections::BTreeSet<String> = handles
        .iter()
        .flat_map(|config| {
            let mut levels = Vec::new();
            let mut entries = config.entries(None).unwrap();
            while let Some(entry) = entries.next() {
                levels.push(format!("{:?}", entry.unwrap().level()));
            }
            levels
        })
        .collect();
    assert_eq!(
        levels,
        std::collections::BTreeSet::from(["Local".to_owned()])
    );
}

#[test]
fn an_interrupted_landing_uses_the_durable_decision_on_reopen() {
    for recorded in [crate::LandingRecord::Absent, crate::LandingRecord::Reviewed] {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let before = trees.repo.head().unwrap().target().unwrap();
        let lease = trees.claim(&name("node"), &[]).unwrap();
        std::fs::write(lease.path().join("lab/notes.md"), b"candidate").unwrap();
        let candidate = Checkpoint::open(lease.path())
            .unwrap()
            .land(&[], TimeMs::new(2_000), &owner(), "offer")
            .unwrap();
        let plan = trees.plan_reviewed(lease.name(), &candidate.oid).unwrap();
        let mut intent = String::new();
        let interrupted = plan.before_landing(&landing(3_000, &owner(), false), |merge| {
            intent = merge;
            Err(kernel::AxError::failure(
                kernel::AxCode::StorageFatal,
                "interrupt landing",
                "fixture",
            )
            .with_recovery("reopen the project"))
        });
        assert!(interrupted.is_err());
        assert_eq!(trees.repo.head().unwrap().target(), Some(before));
        drop(trees);
        let reopened = Worktrees::open(dir.path()).unwrap();
        reopened
            .reconcile_landing(|point| {
                assert_eq!(point.to_string(), intent);
                Ok(recorded)
            })
            .unwrap();
        reopened
            .reconcile_landing(|_| panic!("the pending intent was already consumed"))
            .unwrap();
        let tip = reopened.repo.head().unwrap().target().unwrap();
        assert_eq!(tip == before, recorded == crate::LandingRecord::Absent);
        assert_eq!(
            reopened
                .repo
                .refname_to_id(&format!("refs/sprawling/reviewed/{intent}"))
                .is_ok(),
            recorded == crate::LandingRecord::Reviewed
        );
    }
}
