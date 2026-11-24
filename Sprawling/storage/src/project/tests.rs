// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::{Landing, ModelChoice, Provenance, WorktreeName};
use kernel::{B3Hash, GitOid, RunId, TimeMs};

fn actor(address: &Address, run: RunId) -> Provenance {
    Provenance::new(
        run,
        address.clone(),
        B3Hash::digest(b"private workspace"),
        ModelChoice {
            id: String::new(),
            effort: None,
        },
    )
}

fn commit(repo: &git2::Repository, paths: &[&str]) -> git2::Oid {
    let mut index = repo.index().unwrap();
    for path in paths {
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig =
        git2::Signature::new("tester", "tester@example.invalid", &git2::Time::new(1, 0)).unwrap();
    let parent = repo.head().ok().map(|head| head.peel_to_commit().unwrap());
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn a_locked_source_and_an_independent_native_commit_do_not_own_a_run_or_its_review() {
    let home = tempfile::tempdir().unwrap();
    let city = home.path().join("private");
    std::fs::create_dir(&city).unwrap();
    let source = home.path().join("workspace").join("mobile");
    let native = git2::Repository::init(&source).unwrap();
    let addr = Address::parse("mobile").unwrap();
    let project = ProjectRepository::at(&city, &addr).unwrap();
    let run = RunId::from_bytes([1; 16]);
    let who = actor(&addr, run);
    std::fs::write(source.join("notes"), b"committed").unwrap();
    let base = commit(&native, &["notes"]);
    let upstream = native.head().unwrap().shorthand().unwrap().to_owned();
    let lock = native.path().join("index.lock");
    std::fs::write(&lock, b"other editor").unwrap();
    std::fs::write(source.join("notes"), b"dirty").unwrap();
    let snapshot =
        SourceSnapshot::read(&city, &source, GitOid::parse(&base.to_string()).as_ref()).unwrap();
    std::fs::create_dir(city.join("mobile")).unwrap();
    std::fs::copy(snapshot.path().join("notes"), city.join("mobile/notes")).unwrap();
    let scopes = vec!["mobile".to_owned()];
    let first = snapshot
        .checkpoint(
            project.checkpoint().unwrap(),
            TimeMs::new(1),
            &who,
            &mut |_| {},
        )
        .unwrap();
    snapshot.bind(&project, first.oid).unwrap();
    let lease = project.claim(run, &scopes).unwrap();
    assert_eq!(
        std::fs::read(lease.path().join("mobile/notes")).unwrap(),
        b"committed"
    );
    let native_run = git2::Repository::open(lease.path()).unwrap();
    std::fs::write(lease.path().join("mobile/notes"), b"reviewed").unwrap();
    std::fs::write(lease.path().join("foreign"), b"outside the project").unwrap();
    std::fs::create_dir_all(lease.path().join("mobile/Archive/decision")).unwrap();
    std::fs::write(
        lease
            .path()
            .join("mobile/Archive/decision")
            .join("0-private.md"),
        b"private conversation",
    )
    .unwrap();
    std::fs::write(lease.path().join("mobile/Handoff.md"), b"private handoff").unwrap();
    commit(&native_run, &["foreign"]);
    let mut writer = ProjectRepository::run_checkpoint(&city, lease.path(), run, &addr).unwrap();
    let candidate = writer.land(&scopes, TimeMs::new(2), &who, "offer").unwrap();
    writer.close_writer().unwrap();
    let trees = project.trees().unwrap();
    let plan = trees
        .plan_reviewed(
            &WorktreeName::parse(lease.name().as_str()).unwrap(),
            &candidate.oid,
        )
        .unwrap();
    let merge = plan
        .apply(&Landing {
            t: TimeMs::new(3),
            of: &who,
            subject: "accepted",
            reviewed_by_person: false,
        })
        .unwrap();
    let approved = GitOid::parse(&merge).unwrap();
    assert!(
        project
            .publish(candidate.oid, &source, &upstream, "contribution")
            .is_err()
    );
    std::fs::write(source.join("unrelated"), b"native development").unwrap();
    let mut changed = native
        .treebuilder(Some(&native.find_commit(base).unwrap().tree().unwrap()))
        .unwrap();
    changed
        .insert(
            "unrelated",
            native.blob(b"native development").unwrap(),
            0o100644,
        )
        .unwrap();
    let old = native.find_commit(base).unwrap();
    let advanced = native
        .commit(
            Some("HEAD"),
            &old.author(),
            &old.committer(),
            "independent",
            &native.find_tree(changed.write().unwrap()).unwrap(),
            &[&old],
        )
        .unwrap();

    let published = project
        .publish(approved, &source, &upstream, "contribution")
        .unwrap();
    assert_eq!(
        project
            .publish(approved, &source, &upstream, "contribution")
            .unwrap(),
        published
    );
    let result = native
        .find_commit(git2::Oid::from_str(&published).unwrap())
        .unwrap();
    let tree = result.tree().unwrap();
    assert_eq!(result.parent_id(0).unwrap(), advanced);
    assert!(tree.get_name("unrelated").is_some());
    assert!(tree.get_name("foreign").is_none());
    assert!(tree.get_name("Archive").is_none());
    assert!(tree.get_name("Handoff.md").is_none());
    assert_eq!(
        native
            .find_blob(tree.get_name("notes").unwrap().id())
            .unwrap()
            .content(),
        b"reviewed"
    );
    assert_eq!(native.head().unwrap().target(), Some(advanced));
    assert_eq!(std::fs::read(source.join("notes")).unwrap(), b"dirty");
    assert_eq!(std::fs::read(lock).unwrap(), b"other editor");
    trees.release(lease).unwrap();
}

#[test]
fn a_redirected_private_git_directory_cannot_claim_a_native_project() {
    let home = tempfile::tempdir().unwrap();
    let native = git2::Repository::init(home.path().join("source")).unwrap();
    let city = home.path().join("city");
    let addr = Address::parse("project").unwrap();
    let project = ProjectRepository::at(&city, &addr).unwrap();
    std::fs::create_dir_all(project.root()).unwrap();
    std::fs::write(
        project.root().join(".git"),
        format!("gitdir: {}\n", native.path().display()),
    )
    .unwrap();
    assert!(ProjectRepository::at(&city, &addr).is_err());
}
