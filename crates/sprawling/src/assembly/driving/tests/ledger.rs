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

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;
use crate::views::Views;

#[test]
fn what_a_run_changes_is_changed_after_the_line_that_announces_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let lab = dir.path().join("lab");
    std::fs::create_dir_all(lab.join("room1")).unwrap();
    std::fs::write(lab.join(city::ROADMAP_FILE), PLAN_TWO_FREE_ROWS).unwrap();

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "remembering why",
                "tu_1",
                "archive",
                serde_json::json!({
                    "action": "record",
                    "kind": "decision",
                    "text": "we chose the embedded store",
                }),
            ),
            tool_completion(
                "taking a row",
                "tu_2",
                "plan",
                serde_json::json!({ "action": "claim", "node": "1" }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    // Which line landed, and whether the city already carried what
    // it announces at that moment.
    let seen: Arc<std::sync::Mutex<Vec<(&'static str, bool)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let watch = Arc::clone(&seen);
    let root = dir.path().to_path_buf();
    worker.observe(Box::new(move |record: &EventRecord| {
        let lab = Address::parse("lab").unwrap();
        let carried = match record.kind() {
            EventKind::AssetArchived => Some((
                "asset_archived",
                !city::archive_index(&root, &lab).unwrap().is_empty(),
            )),
            EventKind::RoadmapClaimed => Some((
                "roadmap_claimed",
                city::roadmap(&root, &lab).unwrap().contains("In progress"),
            )),
            _ => None,
        };
        if let Some(landed) = carried {
            watch.lock().unwrap().push(landed);
        }
    }));

    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "remember one thing and take one row".to_owned(),
            goal: "one decision, one claim".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"order"),
            session: None,
            effort: None,
        })
        .unwrap();
    drop(provider);

    let landed = seen.lock().unwrap().clone();
    assert_eq!(
        landed.len(),
        2,
        "both effects reached the history: {landed:?}"
    );
    let early: Vec<&str> = landed
        .iter()
        .filter(|(_, already)| *already)
        .map(|(line, _)| *line)
        .collect();
    assert!(
        early.is_empty(),
        "the city already carried what these lines announce before they were on the ledger: \
         {early:?}"
    );

    // Ordered, not lost: both changes are the city's once the run
    // is over.
    let lab_addr = Address::parse("lab").unwrap();
    assert_eq!(
        city::archive_index(dir.path(), &lab_addr).unwrap().len(),
        1,
        "the decision is on the shelf"
    );
    assert!(
        city::roadmap(dir.path(), &lab_addr)
            .unwrap()
            .contains("| 1 | wire the kiln | 1 |  | In progress |"),
        "the node is claimed in the plan"
    );
}
#[test]
fn a_resident_crosses_two_runs_with_the_same_identity_segment() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    std::fs::write(
        city::urbanite_path(dir.path(), &addr),
        "# URBANITE.md\n\nAsks rather than guesses.\n",
    )
    .unwrap();

    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let segments = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&segments);
    worker.observe(Box::new(move |record: &EventRecord| {
        if record.kind() == EventKind::ModelCalled
            && let Some(list) = record.data().as_map().get("segments")
        {
            sink.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(list.clone());
        }
    }));

    for task in ["first errand", "second errand"] {
        worker
            .handle(channels::Command::Dispatch {
                addr: addr.clone(),
                task: task.to_owned(),
                goal: "one turn is enough".to_owned(),
                mode: channels::ModeTag::parse("plan").unwrap(),
                budget: kernel::BudgetCap::default(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, task.as_bytes()),
                session: None,
                effort: None,
            })
            .unwrap();
    }

    let seen = segments
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(seen.len() >= 2, "two runs, two model calls at least");
    // The resident segment is index 2 of the four; the run segment
    // differs between the two errands, the resident one does not.
    let first = seen[0].as_array().unwrap();
    let second = seen[seen.len() - 1].as_array().unwrap();
    assert_eq!(
        first[2], second[2],
        "the same resident reads the same instructions on every run"
    );
    assert_ne!(first[3], second[3], "and each run carries its own job");
}
#[test]
fn the_views_answer_from_the_ledger_and_rebuild_to_the_same_answer() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let live = std::sync::Arc::new(std::sync::Mutex::new(Views::new(dir.path())));
    let folding = std::sync::Arc::clone(&live);
    worker.observe(Box::new(move |record: &EventRecord| {
        folding
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .apply(record)
            .unwrap();
    }));
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say hello".to_owned(),
            goal: "one turn is enough".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let mut live = live
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let channels::Answer::City(city) = live.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert_eq!(city.frozen, 1, "the dispatched run reached its freeze");
    let run = city.runs.iter().find(|row| row.frozen).unwrap().run;
    let channels::Answer::Run(Some(one)) = live.answer(&channels::Query::RunView { run }) else {
        panic!("RunView answers about a run the city has");
    };
    assert_eq!(one.last_kind, EventKind::RunFrozen);

    // The same answer arrives from a cold rebuild: a view is
    // disposable exactly to the extent that this holds.
    let mut rebuilt = rebuild_views(&report.ledger_dir).unwrap();
    let channels::Answer::City(again) = rebuilt.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert_eq!(city, again);

    // Every query this build carries now answers; the shape that
    // named itself unavailable is gone because nothing is left to
    // name. What a page must still be able to tell apart is "this
    // city archived nothing" from "this build cannot say", and the
    // first is an empty list rather than a refusal.
    let channels::Answer::Registry(registry) = live.answer(&channels::Query::RegistryView) else {
        panic!("RegistryView answers with a registry");
    };
    assert!(registry.assets.is_empty());
}
