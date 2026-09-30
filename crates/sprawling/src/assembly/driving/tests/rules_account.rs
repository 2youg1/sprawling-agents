// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

use super::flight::{asked, history};

/// The digest the account writes for a document's bytes.
fn digest(text: &str) -> String {
    kernel::B3Hash::digest(text.as_bytes()).to_string()
}

/// The entries the account holds for one governing document, oldest
/// first. Selected by scope and document because a building holds two:
/// the scope alone cannot say which one moved.
fn books<'a>(
    lines: &'a [serde_json::Value],
    scope: &str,
    which: &str,
) -> Vec<&'a serde_json::Value> {
    lines
        .iter()
        .filter(|line| {
            line["kind"] == "rules_changed"
                && line["data"]["scope"] == scope
                && line["data"]["which"] == which
        })
        .collect()
}

/// A hand that reached `RULES.toml` while nothing was looking is in the
/// account before the run it governs stands up, and the first sight of
/// a document opens its account rather than claiming a change.
#[test]
fn a_run_standing_up_books_the_rules_it_will_stand_under() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::assembly::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules("review = false\n"));
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let moved = ordinary_rules("review = true\n");
    std::fs::write(
        city::rules_path(dir.path(), &Address::parse("lab").unwrap()),
        &moved,
    )
    .unwrap();
    let (staged, _continuation) = worker
        .stage_dispatch(
            asked("lab/east"),
            "fire the kiln".to_owned(),
            "the kiln is fired".to_owned(),
        )
        .unwrap();
    drop(staged);

    let lines = history(&report.ledger_dir);
    let entries = books(&lines, "building:lab", "RULES.toml");
    assert_eq!(entries.len(), 1, "one hand, one entry: {entries:?}");
    assert_eq!(
        entries[0]["data"],
        serde_json::json!({
            "scope": "building:lab",
            "which": "RULES.toml",
            "after": digest(&moved),
            "bytes": moved.len(),
        }),
        "no `before`: nothing booked this document yet"
    );
}

/// One entry's `after` is the next entry's `before`, every entry lands
/// before the first line of the run it governs, and a restart folds the
/// same account out of the lines alone.
#[test]
fn a_rules_change_says_which_entry_it_reached_the_city_in() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::assembly::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("west")).unwrap();
    let stood = ordinary_rules("review = false\n");
    lay_rules(dir.path(), "lab", &stood);
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("east is done", None),
            completion("west is done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let dispatch = |worker: &mut RunWorker, room: &str| {
        let run = worker
            .dispatch_into_lane(
                asked(room),
                format!("fire the kiln in {room}"),
                format!("the kiln in {room} is fired"),
                Owing::asked(wire::Reply::nowhere()),
            )
            .unwrap();
        worker.land_the_rest().unwrap();
        run
    };

    let first = dispatch(&mut worker, "lab/east");
    let moved = ordinary_rules("review = true\n");
    std::fs::write(
        city::rules_path(dir.path(), &Address::parse("lab").unwrap()),
        &moved,
    )
    .unwrap();
    let second = dispatch(&mut worker, "lab/west");
    drop(provider);

    let lines = history(&report.ledger_dir);
    let entries = books(&lines, "building:lab", "RULES.toml");
    let chain: Vec<_> = entries
        .iter()
        .map(|entry| {
            (
                entry["data"]["before"].clone(),
                entry["data"]["after"].clone(),
            )
        })
        .collect();
    assert_eq!(
        chain,
        vec![
            (serde_json::Value::Null, digest(&stood).into()),
            (digest(&stood).into(), digest(&moved).into()),
        ],
        "the opening line, then the hand that moved it"
    );
    for (entry, run) in entries.iter().zip([first, second]) {
        let booked = entry["seq"].as_u64().unwrap();
        let run = run.to_string();
        let stood_up = lines
            .iter()
            .filter(|line| line["run"] == run.as_str())
            .filter_map(|line| line["seq"].as_u64())
            .min()
            .unwrap();
        assert!(
            booked < stood_up,
            "booked at {booked}, {run} began at {stood_up}"
        );
    }
    let rebuilt = Standing::fold(&report.ledger_dir).unwrap().governance;
    assert_eq!(
        worker.governance.rules, rebuilt.rules,
        "a restart rebuilds the account"
    );
}

/// A dispatch to a building the city does not have books the city's own
/// layer and opens no account for the building.
#[test]
fn a_building_that_does_not_exist_opens_no_account() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::assembly::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let (staged, _continuation) = worker
        .stage_dispatch(
            asked("ghost/east"),
            "haunt".to_owned(),
            "haunted".to_owned(),
        )
        .unwrap();
    drop(staged);

    let scopes: Vec<_> = history(&report.ledger_dir)
        .into_iter()
        .filter(|line| line["kind"] == "rules_changed")
        .map(|line| (line["data"]["scope"].clone(), line["data"]["which"].clone()))
        .collect();
    assert_eq!(
        scopes,
        vec![("city".into(), "CONFIG.toml".into())],
        "only the city's own layer is booked"
    );
}
