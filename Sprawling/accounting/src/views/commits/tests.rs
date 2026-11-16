// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::{B3Hash, EventDraft, Payload, TimeMs};

fn record(kind: EventKind, data: serde_json::Map<String, serde_json::Value>) -> EventRecord {
    record_at(kind, data, 9, "lab/room1")
}

fn record_at(
    kind: EventKind,
    data: serde_json::Map<String, serde_json::Value>,
    seq: u64,
    addr: &str,
) -> EventRecord {
    record_by(kind, data, (seq, addr), RunId::from_bytes([4u8; 16]))
}

fn record_by(
    kind: EventKind,
    data: serde_json::Map<String, serde_json::Value>,
    (seq, addr): (u64, &str),
    run: RunId,
) -> EventRecord {
    let draft = EventDraft {
        run,
        t: TimeMs::new(7),
        who: "resident".to_owned(),
        addr: Some(Address::parse(addr).unwrap()),
        kind,
        data: Payload::new(data).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
}

fn checkpointed(seq: u64, addr: &str) -> EventRecord {
    let mut data = serde_json::Map::new();
    data.insert(
        "oid".to_owned(),
        serde_json::Value::String(format!("{seq:02x}").repeat(20)),
    );
    record_at(EventKind::CheckpointCommitted, data, seq, addr)
}

fn checkpoint_by(seq: u64, oid: &str, run: u8) -> EventRecord {
    let mut data = serde_json::Map::new();
    data.insert("oid".to_owned(), serde_json::Value::String(oid.to_owned()));
    record_by(
        EventKind::CheckpointCommitted,
        data,
        (seq, "lab/room1"),
        RunId::from_bytes([run; 16]),
    )
}

fn oid_at(seq: u64) -> String {
    format!("{seq:02x}").repeat(20)
}

fn previous_of(views: &super::super::holding::Views, seq: u64) -> Option<wire::CommitAt> {
    views
        .commits_answer(None, None, 20)
        .unwrap()
        .commits
        .into_iter()
        .find(|commit| commit.seq == Seq::new(seq))
        .unwrap()
        .previous
}

#[test]
fn a_commit_carries_the_b3_of_the_line_that_announced_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = super::super::holding::Views::new(dir.path());
    let line = checkpoint_by(3, &oid_at(3), 1);
    views.fold_commit(&line).unwrap();
    let page = views.commits_answer(None, None, 20).unwrap();
    let b3s: Vec<Option<B3Hash>> = page.commits.iter().map(|commit| commit.b3).collect();
    assert_eq!(
        b3s,
        vec![Some(B3Hash::digest(&line.canonical_line().unwrap()))]
    );
}

#[test]
fn a_commit_names_the_one_its_run_announced_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = super::super::holding::Views::new(dir.path());
    for (seq, run) in [(3, 1), (5, 2), (8, 1)] {
        views
            .fold_commit(&checkpoint_by(seq, &oid_at(seq), run))
            .unwrap();
    }
    let first = wire::CommitAt {
        oid: GitOid::parse(&oid_at(3)).unwrap(),
        seq: Seq::new(3),
    };
    assert_eq!(
        (
            previous_of(&views, 8),
            previous_of(&views, 5),
            previous_of(&views, 3)
        ),
        (Some(first), None, None),
        "the other run's commit in between is not this run's previous one"
    );
}

#[test]
fn a_page_of_commits_carries_each_ones_parents_from_git() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let root = repo
        .commit(Some("HEAD"), &signature, &signature, "root", &tree, &[])
        .unwrap();
    let parent = repo.find_commit(root).unwrap();
    let child = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "child",
            &tree,
            &[&parent],
        )
        .unwrap();
    let mut views = super::super::holding::Views::new(dir.path());
    for (seq, oid) in [
        (3, root.to_string()),
        (5, child.to_string()),
        (8, oid_at(8)),
    ] {
        views.fold_commit(&checkpoint_by(seq, &oid, 1)).unwrap();
    }
    let asked = wire::Query::Commits {
        building: None,
        before: None,
        limit: 20,
    };
    let wire::Answer::Commits(page) = views.prepare(&asked).finish() else {
        panic!("a page of commits");
    };
    let parents: Vec<(u64, Option<Vec<String>>)> = page
        .commits
        .iter()
        .map(|commit| {
            let named = commit
                .parents
                .as_ref()
                .map(|oids| oids.iter().map(ToString::to_string).collect());
            (commit.seq.value(), named)
        })
        .collect();
    assert_eq!(
        parents,
        vec![
            (8, None),
            (5, Some(vec![root.to_string()])),
            (3, Some(Vec::new())),
        ],
        "a commit the repository does not hold has no parents to read; a root has none"
    );
}

#[test]
fn a_page_of_commits_carries_each_ones_message_from_git() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let said = "checkpoint: wave 2

Sprawling-Actor: lab/room1
";
    let made = repo
        .commit(Some("HEAD"), &signature, &signature, said, &tree, &[])
        .unwrap();
    let mut views = super::super::holding::Views::new(dir.path());
    views
        .fold_commit(&checkpoint_by(3, &made.to_string(), 1))
        .unwrap();
    views.fold_commit(&checkpoint_by(5, &oid_at(5), 1)).unwrap();
    let asked = wire::Query::Commits {
        building: None,
        before: None,
        limit: 20,
    };
    let wire::Answer::Commits(page) = views.prepare(&asked).finish() else {
        panic!("a page of commits");
    };
    let messages: Vec<(u64, Option<String>)> = page
        .commits
        .into_iter()
        .map(|commit| (commit.seq.value(), commit.message))
        .collect();
    assert_eq!(messages, vec![(5, None), (3, Some(said.to_owned()))]);
}

fn addr(text: &str) -> Address {
    Address::parse(text).unwrap()
}

#[test]
fn a_buildings_commits_are_listed_newest_first_and_paged_by_seq() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = super::super::holding::Views::new(dir.path());
    for record in [
        checkpointed(3, "lab/room1"),
        checkpointed(5, "hall/mayor"),
        checkpointed(8, "lab"),
    ] {
        views.fold_commit(&record).unwrap();
    }

    let lab = views.commits_answer(Some(&addr("lab")), None, 20).unwrap();
    let seqs: Vec<u64> = lab.commits.iter().map(|c| c.seq.value()).collect();
    assert_eq!(
        seqs,
        vec![8, 3],
        "the building root and its room, newest first"
    );
    assert!(!lab.more);
    assert_eq!(lab.building, Some(addr("lab")));

    let first = views.commits_answer(Some(&addr("lab")), None, 1).unwrap();
    assert_eq!(first.commits.len(), 1);
    assert!(first.more, "one page held back is a page");
    let next = views
        .commits_answer(Some(&addr("lab")), Some(Seq::new(8)), 1)
        .unwrap();
    assert_eq!(next.commits.first().map(|c| c.seq.value()), Some(3));
    assert!(!next.more);
    assert_eq!(next.before, Some(Seq::new(8)));

    let hall = views.commits_answer(Some(&addr("hall")), None, 20).unwrap();
    assert_eq!(hall.commits.len(), 1);
    assert_eq!(hall.commits.first().unwrap().actor.as_str(), "hall/mayor");

    assert!(
        views
            .commits_answer(Some(&addr("la")), None, 20)
            .unwrap()
            .commits
            .is_empty()
    );

    let all = views.commits_answer(None, None, 20).unwrap();
    assert_eq!(all.commits.len(), 3);
}

#[test]
fn the_line_that_pins_a_job_is_not_a_commit() {
    let mut pin = serde_json::Map::new();
    pin.insert(
        "job".to_owned(),
        serde_json::Value::String(
            "file:hall/mayor@0123456789abcdef0123456789abcdef01234567".to_owned(),
        ),
    );
    assert!(
        commit_facts(&record(EventKind::CheckpointCommitted, pin))
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_merge_names_its_commit_under_its_own_key() {
    let oid = "cd".repeat(20);
    let mut merged = serde_json::Map::new();
    merged.insert("commit".to_owned(), serde_json::Value::String(oid.clone()));
    let (found, facts) = commit_facts(&record(EventKind::PrMerged, merged))
        .unwrap()
        .unwrap();
    assert_eq!(found.to_string(), oid);

    let said = facts.answer(found, vec![facts.run], UsdMicros::new(0));
    assert_eq!(said.model, String::new());
    assert_eq!(said.lineage.len(), 1, "a first run is its own lineage");
    assert_eq!(said.effort, None);
    assert_eq!(said.session.unwrap().as_str(), "room1");
}

#[test]
fn a_run_at_a_buildings_own_address_has_no_session() {
    assert!(session_of(&Address::parse("lab").unwrap()).is_none());
}
