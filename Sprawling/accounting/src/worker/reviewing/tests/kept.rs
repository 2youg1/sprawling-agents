// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn two_runs_in_one_room_under_review_work_in_one_tree() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules("review = true\n"));
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (task, idem) in [("first", b"first".as_slice()), ("second", b"second")] {
        worker
            .handle(wire::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: task.to_owned(),
                goal: task.to_owned(),
                model: None,
                policy: kernel::RunPolicy::of(kernel::Mode::Work),
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

#[test]
fn a_tree_left_locked_by_a_dead_writer_is_lent_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
        worker.handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "work".to_owned(),
            goal: "work".to_owned(),
            model: None,
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
            session: None,
            effort: None,
        })
    };
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"first").unwrap();
    drop(worker);
    let name = storage::WorktreeName::parse(&trees_opened(&report.ledger_dir)[0]).unwrap();

    drop(
        storage::Worktrees::open(dir.path())
            .unwrap()
            .claim(&name, &["lab".to_owned()])
            .unwrap(),
    );

    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"second").unwrap();
    assert_eq!(trees_opened(&report.ledger_dir).len(), 2);
}

#[test]
fn a_review_dispatch_sent_again_after_a_restart_is_answered_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
        vec![
            completion("first", None),
            completion("second", None),
            completion("third", None),
        ],
    );

    let dispatch = |worker: &mut RunWorker, idem: &[u8]| {
        worker.serve_one(Posted {
            command: wire::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "work".to_owned(),
                goal: "work".to_owned(),
                policy: kernel::RunPolicy::of(kernel::Mode::Work),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
                session: None,
                effort: None,
                model: None,
            },
            reply: wire::Reply::nowhere(),
        });
        worker.land_the_rest().unwrap();
    };
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"books the rules");
    dispatch(&mut worker, b"sent twice");
    drop(worker);

    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    dispatch(&mut worker, b"sent twice");
    assert_eq!(
        trees_opened(&report.ledger_dir).len(),
        2,
        "the command sent again after the restart started no run"
    );
}

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

#[test]
fn a_tree_that_waits_on_the_index_lock_is_placed_in_the_lane() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules("review = true\n"));
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    storage::Checkpoint::open(dir.path()).unwrap();
    let lock = dir.path().join(".git").join("index.lock");
    std::fs::write(&lock, "").unwrap();

    worker.serve_one(Posted {
        command: wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "work".to_owned(),
            goal: "work".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"locked"),
            session: None,
            effort: None,
            model: None,
        },
        reply: wire::Reply::nowhere(),
    });
    let in_the_air = worker.flight.in_flight();
    let landed = worker.land_the_rest();
    std::fs::remove_file(&lock).unwrap();

    assert_eq!(
        in_the_air, 1,
        "the desk took the dispatch off while the index was locked"
    );
    assert!(
        landed.is_err(),
        "and the lane that could not place the tree came home refused"
    );
}

#[test]
fn a_lane_that_fails_after_placing_the_tree_gives_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();

    lay_rules(
        dir.path(),
        "lab",
        &shut_rules("review = true\nprefixes = [\"elsewhere\"]\n"),
    );
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let landed: Vec<bool> = [b"first".as_slice(), b"second"]
        .into_iter()
        .map(|idem| {
            worker.serve_one(Posted {
                command: wire::Command::Dispatch {
                    addr: Address::parse("lab/room1").unwrap(),
                    task: "work".to_owned(),
                    goal: "work".to_owned(),
                    policy: kernel::RunPolicy::of(kernel::Mode::Work),
                    idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
                    session: None,
                    effort: None,
                    model: None,
                },
                reply: wire::Reply::nowhere(),
            });
            worker.land_the_rest().is_err()
        })
        .collect();

    assert_eq!(
        (landed, trees_opened(&report.ledger_dir).len()),
        (vec![true, true], 2),
        "each run placed the room's tree, failed, and gave the tree back"
    );
}
