// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The checks derived from `crates/storage/spec/Checkpoint/Concurrent.lean`
//! §8-39 "验收": real threads, one city, no lock around a checkpoint.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Barrier};

use kernel::{Address, B3Hash, RunId, TimeMs};

use super::super::provenance::{ModelChoice, Provenance};
use super::*;

fn writer(n: u8, room: &str) -> Provenance {
    Provenance::new(
        RunId::parse(&format!("018f5b2a-0000-7000-8000-0000000000{n:02}")).unwrap(),
        Address::parse(room).unwrap(),
        B3Hash::digest(b"a city"),
        ModelChoice {
            id: "test-model".to_owned(),
            effort: None,
        },
    )
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn references(root: &Path) -> BTreeSet<String> {
    let repo = git2::Repository::open(root).unwrap();
    repo.references()
        .unwrap()
        .map(|r| r.unwrap().name().unwrap().to_owned())
        .collect()
}

/// The paths and contents a commit's tree holds.
fn tree_of(root: &Path, oid: &str) -> BTreeSet<(String, String)> {
    let repo = git2::Repository::open(root).unwrap();
    let tree = repo
        .find_commit(git2::Oid::from_str(oid).unwrap())
        .unwrap()
        .tree()
        .unwrap();
    let mut held = BTreeSet::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if let Some(blob) = entry.to_object(&repo).unwrap().as_blob() {
            held.insert((
                format!("{dir}{}", entry.name().unwrap()),
                String::from_utf8_lossy(blob.content()).into_owned(),
            ));
        }
        git2::TreeWalkResult::Ok
    })
    .unwrap();
    held
}

fn oid_of(payload: &kernel::Payload) -> String {
    payload.as_map()["oid"].as_str().unwrap().to_owned()
}

/// `others_never_touch_an_index`, `a_pin_holds_what_its_writer_staged`: two
/// writers checkpoint their own rooms of one city on two threads, wave for
/// wave in lockstep, and each commit's tree is exactly what its own writer
/// staged; both references stand.
#[test]
fn two_writers_checkpoint_one_city_at_once_and_each_tree_is_its_own() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    Checkpoint::open(&root).unwrap();
    let start = Arc::new(Barrier::new(2));
    let waves: u32 = 8;
    let spawn = |n: u8, room: &'static str| {
        let root = root.clone();
        let start = Arc::clone(&start);
        std::thread::spawn(move || {
            let of = writer(n, room);
            let mut own = Checkpoint::open_writer(&root, of.run()).unwrap();
            let mut made = Vec::new();
            for wave in 0..waves {
                let body = format!("{room} wave {wave}");
                write(&root, &format!("{room}/file.txt"), &body);
                start.wait();
                let payload = own
                    .wave_pre(&[room.to_owned()], TimeMs::new(1_000), &of)
                    .unwrap();
                made.push((oid_of(&payload), body));
            }
            own.close_writer().unwrap();
            (of, made)
        })
    };
    let first = spawn(1, "work/one");
    let second = spawn(2, "work/two");
    for handle in [first, second] {
        let (of, made) = handle.join().unwrap();
        let room = of.actor().as_str().to_owned();
        let filed = references(&root);
        for (oid, body) in made {
            assert_eq!(
                tree_of(&root, &oid),
                BTreeSet::from([(format!("{room}/file.txt"), body)])
            );
            assert!(filed.contains(&format!("refs/sprawling/runs/{}/{oid}", of.run())));
        }
    }
    assert_eq!(Checkpoint::sweep_writers(&root).unwrap(), 0);
}

/// `before_the_pin_no_reference_moves`, `a_crash_reaches_nothing_new`: a
/// writer that staged and wrote its tree's objects, then died before its
/// commit and pin, leaves the city's references as they were; the index it
/// left is swept when the city opens again.
#[test]
fn a_writer_lost_before_its_pin_leaves_every_reference_where_it_was() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "work/one/file.txt", "before");
    let of = writer(1, "work/one");
    let mut base = Checkpoint::open(root).unwrap();
    base.ensure_base(&["work/one".to_owned()], TimeMs::new(1_000), &of)
        .unwrap();
    let before = references(root);
    let head = git2::Repository::open(root)
        .unwrap()
        .head()
        .unwrap()
        .target();

    write(root, "work/one/file.txt", "after");
    let lost = writer(2, "work/one");
    let mut dying = Checkpoint::open_writer(root, lost.run()).unwrap();
    dying.stage_scopes(&["work/one".to_owned()]).unwrap();
    dying.repo.index().unwrap().write_tree().unwrap();
    drop(dying);

    assert_eq!(references(root), before);
    assert_eq!(
        git2::Repository::open(root)
            .unwrap()
            .head()
            .unwrap()
            .target(),
        head
    );
    assert_eq!(Checkpoint::sweep_writers(root).unwrap(), 1);
}

/// `a_lost_base_race_is_the_serial_order`: writers that all see a city with
/// no commit and all ask for its base leave exactly one commit at HEAD; every
/// other one answers that there was nothing to make.
#[test]
fn writers_racing_for_the_base_make_exactly_one() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    write(&root, "work/one/file.txt", "a");
    Checkpoint::open(&root).unwrap();
    let racers: u8 = 4;
    let start = Arc::new(Barrier::new(usize::from(racers)));
    let handles: Vec<_> = (1..=racers)
        .map(|n| {
            let root = root.clone();
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                let of = writer(n, "work/one");
                let mut handle = Checkpoint::open(&root).unwrap();
                start.wait();
                handle
                    .ensure_base(&["work/one".to_owned()], TimeMs::new(1_000), &of)
                    .unwrap()
                    .map(|payload| oid_of(&payload))
            })
        })
        .collect();
    let made: Vec<String> = handles
        .into_iter()
        .filter_map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(made.len(), 1);
    let head = git2::Repository::open(&root)
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap()
        .to_string();
    assert_eq!(made, vec![head]);
}
