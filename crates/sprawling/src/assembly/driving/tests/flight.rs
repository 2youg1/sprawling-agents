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

/// A dispatch as the desk delivers one: a person's work, sent to a room
/// that already exists.
fn asked(addr: &str) -> Assignment {
    Assignment {
        addr: Address::parse(addr).unwrap(),
        session: None,
        effort: None,
        mode: runtime::Mode::PlanGoal,
        parent: None,
        succession: None,
    }
}

/// Every line of the city's history, parsed, oldest first.
fn history(ledger_dir: &std::path::Path) -> Vec<serde_json::Value> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

/// **Two pieces of work a person sent go into two lanes**, and the
/// accounting thread lands both.
///
/// The assertion that bites is the last one: driven one at a time, the
/// second run's first line comes after the first run has frozen, so the
/// set of runs that had started by then is one rather than two.
#[test]
fn two_dispatches_from_the_desk_drive_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("west")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_TWO_FREE_ROWS,
    )
    .unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("east is done", None),
            completion("west is done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    // Both drives are in the air before either is landed: this is the
    // door the desk takes, and the loop that lands them is below.
    worker
        .dispatch_into_lane(
            asked("lab/east"),
            "fire the east kiln".to_owned(),
            "the east kiln is fired".to_owned(),
            channels::Reply::nowhere(),
        )
        .unwrap();
    worker
        .dispatch_into_lane(
            asked("lab/west"),
            "fire the west kiln".to_owned(),
            "the west kiln is fired".to_owned(),
            channels::Reply::nowhere(),
        )
        .unwrap();
    assert!(worker.driving(), "both drives are in the air");
    worker.land_the_rest().unwrap();
    drop(provider);

    let lines = history(&report.ledger_dir);
    // The chain itself: `verify_ledger_dir` refuses a history whose
    // seq or prev disagree, so reaching this line is that assertion.
    let seqs: Vec<u64> = lines
        .iter()
        .map(|line| line["seq"].as_u64().unwrap())
        .collect();
    assert!(
        seqs.windows(2).all(|pair| pair[1] > pair[0]),
        "two lanes write one ledger, and its seq only ever climbs: {seqs:?}"
    );

    // Each run's own order, which the crossing is what guarantees: a
    // relay that answered on hand-off would let a call reach the
    // history before the run that made it.
    for kind in ["run_started", "model_called"] {
        let count = lines.iter().filter(|line| line["kind"] == kind).count();
        assert!(count >= 2, "both runs wrote {kind}: {count} lines");
    }
    for (at, line) in lines.iter().enumerate() {
        if line["kind"] != "model_called" {
            continue;
        }
        let run = line["run"].as_str().unwrap();
        let started = lines
            .iter()
            .position(|other| other["kind"] == "run_started" && other["run"] == run)
            .expect("a run that called a model said it started");
        assert!(
            started < at,
            "{run} called a model before the line that says it started"
        );
    }

    // What serial driving cannot do: by the time the first run freezes,
    // the second has already started.
    let started_before_the_first_freeze: std::collections::BTreeSet<String> = lines
        .iter()
        .take_while(|line| line["kind"] != "run_frozen")
        .filter(|line| line["kind"] == "run_started")
        .map(|line| line["run"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        started_before_the_first_freeze.len(),
        2,
        "both runs were going before either finished: {started_before_the_first_freeze:?}"
    );
    let frozen = lines
        .iter()
        .filter(|line| line["kind"] == "run_frozen")
        .count();
    assert_eq!(frozen, 2, "every run the desk started has to freeze");
}

/// A city that is closing waits for its lanes. A lane abandoned on an
/// append loses lines the city had already told it were durable, and
/// the handoff would then describe a history that is missing its end.
#[test]
fn a_closing_city_lands_the_runs_still_driving() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_ONE_FREE_ROW,
    )
    .unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .dispatch_into_lane(
            asked("lab/east"),
            "fire the kiln".to_owned(),
            "the kiln is fired".to_owned(),
            channels::Reply::nowhere(),
        )
        .unwrap();
    assert!(worker.driving(), "a lane is going");
    worker.land_the_rest().unwrap();
    assert!(!worker.driving(), "the closing city waited for it");
    worker.close_city().unwrap();
    drop(provider);

    let lines = history(&report.ledger_dir);
    let last = lines.last().expect("a city that closed wrote something");
    assert_eq!(
        last["kind"], "handoff_written",
        "the handoff is the last line, not a line in the middle of a run"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line["kind"] == "run_frozen")
            .count(),
        1,
        "the run in the lane was landed before the city closed"
    );
}
