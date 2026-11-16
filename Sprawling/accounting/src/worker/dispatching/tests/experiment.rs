// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;

/// An experiment lands nothing, and in a building that asks for no
/// review that is held by where it writes: the room's own worktree, so
/// the file it creates never appears in the building, and the ledger
/// says which tree and which policy (`crates/sprawling/Spec.lean` §8-133).
#[test]
fn an_experiment_writes_in_a_tree_of_its_own_in_a_building_without_review() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(building.join("room1")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("trying", Some(("tu_1", "lab/room1/trial.md"))),
            completion("tried", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "try a note".to_owned(),
            goal: "one file, kept apart".to_owned(),
            policy: kernel::RunPolicy {
                landing: kernel::LandingPolicy::Experiment,
                ..kernel::RunPolicy::of(kernel::Mode::Work)
            },
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"trial"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    assert!(
        !building.join("room1").join("trial.md").exists(),
        "an experiment's file reached a building that asks for no review"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let records: Vec<EventRecord> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect();
    assert!(
        records
            .iter()
            .any(|record| record.kind() == EventKind::WorktreeOpened),
        "the experiment was lent the room's tree"
    );
    let started = records
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .unwrap();
    assert_eq!(
        started.data().as_map()["policy"]["landing"],
        "experiment",
        "the run's first line says it was an experiment"
    );
}
