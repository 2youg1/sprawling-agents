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

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let opened: Vec<String> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == EventKind::WorktreeOpened)
        .map(|record| record.data().as_map()["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(opened.len(), 2, "each run opened a tree: {opened:?}");
    assert_eq!(
        opened[0], opened[1],
        "the room's second run took its tree back"
    );
}
