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
