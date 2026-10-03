// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, RunId, TimeMs};

use super::*;
use crate::checkpoint::ModelChoice;

fn resident() -> Provenance {
    Provenance::new(
        RunId::CITY,
        Address::parse("shop").unwrap(),
        B3Hash::digest(b"city"),
        ModelChoice {
            id: String::new(),
            effort: None,
        },
    )
}

/// A base checkpoint over a city that already has a HEAD leaves the branch
/// alone and files its commit, and a wave checkpoint on a fresh handle
/// afterwards still writes to disk: the in-memory store left with the
/// handle that held it.
#[test]
fn a_base_checkpoint_beside_history_is_filed_and_leaves_later_checkpoints_on_disk() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("shop")).unwrap();
    std::fs::write(tmp.path().join("shop").join("a.md"), "a").unwrap();
    let mut first = Checkpoint::open(tmp.path()).unwrap();
    first
        .ensure_base(&["other".to_owned()], TimeMs::new(1_000), &resident())
        .unwrap();
    let head = git2::Repository::open(tmp.path())
        .unwrap()
        .head()
        .unwrap()
        .target();

    let mut seen = Vec::new();
    let payload = Checkpoint::open(tmp.path())
        .unwrap()
        .base_checkpoint(
            &["shop".to_owned()],
            TimeMs::new(2_000),
            &resident(),
            &mut |step| seen.push(step),
        )
        .unwrap();
    std::fs::write(tmp.path().join("shop").join("b.md"), "b").unwrap();
    let wave = Checkpoint::open(tmp.path())
        .unwrap()
        .wave_pre(&["shop".to_owned()], TimeMs::new(3_000), &resident())
        .unwrap();

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let oid_of =
        |payload: &Payload| git2::Oid::from_str(payload.as_map()["oid"].as_str().unwrap()).unwrap();
    let filed = repo
        .find_reference(&format!(
            "refs/sprawling/runs/{}/{}",
            RunId::CITY,
            oid_of(&payload)
        ))
        .is_ok();
    assert_eq!(
        (
            repo.head().unwrap().target(),
            filed,
            repo.find_commit(oid_of(&wave)).is_ok(),
            seen.len(),
        ),
        (head, true, true, 2)
    );
}

/// A base checkpoint refused by the staged-secret scan leaves the disk as it
/// found it: no index entry and no HEAD names an object that only the
/// in-memory store held, so a wave checkpoint over the cleaned folder commits.
#[test]
fn a_refused_base_checkpoint_leaves_no_reference_to_objects_it_never_wrote() {
    let tmp = tempfile::tempdir().unwrap();
    let shop = tmp.path().join("shop");
    std::fs::create_dir_all(&shop).unwrap();
    std::fs::write(shop.join("a.md"), "a").unwrap();
    let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
    std::fs::write(shop.join("key.env"), format!("KEY={token}")).unwrap();

    let refused = Checkpoint::open(tmp.path()).unwrap().base_checkpoint(
        &["shop".to_owned()],
        TimeMs::new(1_000),
        &resident(),
        &mut |_| {},
    );
    std::fs::remove_file(shop.join("key.env")).unwrap();
    let wave = Checkpoint::open(tmp.path()).unwrap().wave_pre(
        &["shop".to_owned()],
        TimeMs::new(2_000),
        &resident(),
    );

    assert_eq!(
        (
            matches!(refused, Err(StorageError::SecretEgress { .. })),
            wave.map(|_| ()).map_err(|err| err.to_string()),
        ),
        (true, Ok(()))
    );
}

/// The object files under `.git/objects`: loose objects (one file each under
/// a two-digit directory) and packs. Counted by name, so the count does not
/// depend on the order a platform lists a directory in.
fn object_files(root: &std::path::Path) -> (usize, usize) {
    let objects = root.join(".git").join("objects");
    let names = |dir: &std::path::Path| -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect()
    };
    let dirs = names(&objects);
    let loose = dirs
        .iter()
        .filter(|dir| dir.file_name().is_some_and(|name| name.len() == 2))
        .map(|dir| names(dir).len())
        .sum();
    let packs = names(&objects.join("pack"))
        .iter()
        .filter(|file| file.extension().is_some_and(|ext| ext == "pack"))
        .count();
    (loose, packs)
}

/// The first base of a city writes every object it makes as one pack, at
/// N, 2N and 4N files alike: the cost that grew with the file count was one
/// new loose-object file per blob (storage §8-8).
#[test]
fn a_first_base_writes_one_pack_and_no_loose_object_at_any_file_count() {
    let counts: Vec<(bool, usize, usize)> = [8usize, 16, 32]
        .iter()
        .map(|&n| {
            let tmp = tempfile::tempdir().unwrap();
            let bulk = tmp.path().join("bulk");
            std::fs::create_dir_all(&bulk).unwrap();
            for i in 0..n {
                std::fs::write(bulk.join(format!("{i}.md")), format!("file {i}")).unwrap();
            }
            let made = Checkpoint::open(tmp.path())
                .unwrap()
                .ensure_base(&["bulk".to_owned()], TimeMs::new(1_000), &resident())
                .unwrap();
            let born = git2::Repository::open(tmp.path()).unwrap().head().is_ok();
            let (loose, packs) = object_files(tmp.path());
            (made.is_some() && born, loose, packs)
        })
        .collect();
    assert_eq!(counts, vec![(true, 0, 1); 3]);
}

/// Derived from `a_held_base_moves_head_only_from_what_it_read`
/// (`crates/storage/spec/Checkpoint/Concurrent.lean`): a base that read
/// "no HEAD" and finds a branch another writer created since is refused,
/// and HEAD stays the other writer's commit.
#[test]
fn a_base_that_read_no_head_is_refused_once_another_writer_made_one() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("shop")).unwrap();
    std::fs::write(tmp.path().join("shop").join("a.md"), "a").unwrap();
    let stale = Checkpoint::open(tmp.path()).unwrap();
    Checkpoint::open(tmp.path())
        .unwrap()
        .ensure_base(&["other".to_owned()], TimeMs::new(1_000), &resident())
        .unwrap();
    let head = || {
        git2::Repository::open(tmp.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
    };
    let made = head();

    let refused = stale.write_held(
        &BaseOf {
            scopes: &["shop".to_owned()],
            t: TimeMs::new(2_000),
            of: &resident(),
        },
        BaseTarget::Head,
        &mut |_| {},
    );
    assert_eq!(
        (
            matches!(
                refused,
                Err(StorageError::Checkpoint {
                    op: "move HEAD",
                    ..
                })
            ),
            head(),
        ),
        (true, made)
    );
}
