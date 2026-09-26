// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::assembly::fixture::*;
use crate::assembly::*;

/// A room under review works in one tree across its runs: the second
/// run takes back the tree the first one left instead of checking the
/// whole city out into a new one.
#[test]
fn two_runs_in_one_room_under_review_work_in_one_tree() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules("review = true\n"));
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (task, idem) in [("first", b"first".as_slice()), ("second", b"second")] {
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: task.to_owned(),
                goal: task.to_owned(),
                mode: kernel::Mode::PlanGoal,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
                session: None,
                effort: None,
            })
            .unwrap();
    }

    let opened = trees_opened(&report.ledger_dir);
    assert_eq!(opened.len(), 2, "each run opened a tree: {opened:?}");
    assert_eq!(
        opened[0], opened[1],
        "the room's second run took its tree back"
    );
}

/// A process that died mid-run left its tree locked, and the city has
/// one writer: the next writer lifts that lock instead of refusing the
/// room its tree for good.
#[test]
fn a_tree_left_locked_by_a_dead_writer_is_lent_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        &ordinary_rules(
            "review = true
",
        ),
    );
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let dispatch = |worker: &mut RunWorker, idem: &[u8]| {
        worker.handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "work".to_owned(),
            goal: "work".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
            session: None,
            effort: None,
        })
    };
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"first").unwrap();
    drop(worker);
    let name = memory::WorktreeName::parse(&trees_opened(&report.ledger_dir)[0]).unwrap();
    // The lease a run that never came home still holds.
    drop(
        memory::Worktrees::open(dir.path())
            .unwrap()
            .claim(&name)
            .unwrap(),
    );

    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"second").unwrap();
    assert_eq!(trees_opened(&report.ledger_dir).len(), 2);
}

/// The name of every tree a `worktree_opened` line records, in order.
fn trees_opened(ledger_dir: &std::path::Path) -> Vec<String> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == EventKind::WorktreeOpened)
        .map(|record| record.data().as_map()["name"].as_str().unwrap().to_owned())
        .collect()
}
