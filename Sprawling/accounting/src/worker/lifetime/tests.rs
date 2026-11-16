// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::RunWorker;
use std::io::Write;

#[test]
fn a_torn_tail_is_told_in_the_startup_scan() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let segment =
        storage::ledger_segments_at(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap()
            .pop()
            .unwrap();
    let torn = b"{\"v\":1,\"seq\":99,\"half";
    std::fs::OpenOptions::new()
        .append(true)
        .open(&segment)
        .unwrap()
        .write_all(torn)
        .unwrap();

    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let summary = worker.startup_scan().unwrap().summary();

    assert!(
        summary.contains(&format!("{} byte(s)", torn.len())),
        "the scan must say how much of the tail was cut: {summary}"
    );
}

#[test]
fn a_crash_left_worktree_is_gone_once_the_city_opens() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let signature = git2::Signature::now("city", "city@example.invalid").unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "base", &tree, &[])
        .unwrap();
    let trees = storage::Worktrees::open(dir.path()).unwrap();
    let name = storage::WorktreeName::parse("run-1").unwrap();
    drop(trees.claim(&name, &[]).unwrap());

    drop(
        RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap(),
    );

    assert_eq!(trees.live().unwrap(), Vec::new());
}
