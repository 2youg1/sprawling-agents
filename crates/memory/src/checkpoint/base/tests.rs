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

/// A base fence over a city that already has a HEAD leaves the branch
/// alone and files its commit, and a wave fence on a fresh handle
/// afterwards still writes to disk: the in-memory store left with the
/// handle that held it.
#[test]
fn a_base_fence_beside_history_is_filed_and_leaves_later_fences_on_disk() {
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
        .base_fence(
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
