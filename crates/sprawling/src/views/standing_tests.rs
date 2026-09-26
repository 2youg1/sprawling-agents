// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a page that has just opened must be able to learn from one
//! `city_view`: which room each run works in, and whether the city is
//! stopped (sprawling-SPEC section 8-52).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::tests::{Place, view_record};
use crate::views::Views;
use kernel::{Address, EventKind, RunId};

fn halt_record(seq: u64, state: &str) -> kernel::EventRecord {
    let mut data = serde_json::Map::new();
    data.insert(
        "scope".to_owned(),
        serde_json::Value::String("city".to_owned()),
    );
    data.insert(
        "state".to_owned(),
        serde_json::Value::String(state.to_owned()),
    );
    view_record(
        Place {
            seq,
            run: RunId::CITY,
        },
        EventKind::CityHalted,
        &Address::parse("hall").unwrap(),
        data,
    )
}

/// A page refreshed after `stop the city` used to see a city that
/// looked open, because the halt was a record it had not been sent and
/// a judgement it could not ask for.
#[test]
fn the_city_view_names_the_scopes_a_halt_shut() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    views.apply(&halt_record(1, "halted")).unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert_eq!(city.halted, vec![channels::HaltScope::City]);

    views.apply(&halt_record(2, "released")).unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert!(city.halted.is_empty(), "a released scope is not shut");
}

/// The run list is what a conversation is assembled from, so every run
/// has to say which room it belongs to and when it began.
#[test]
fn a_run_in_the_city_view_says_which_room_it_works_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("hall/mayor").unwrap();
    let run = RunId::from_bytes([9u8; 16]);
    let mut data = serde_json::Map::new();
    data.insert(
        "task".to_owned(),
        serde_json::Value::String("plan the week".to_owned()),
    );
    data.insert(
        "goal".to_owned(),
        serde_json::Value::String("a roadmap".to_owned()),
    );
    views
        .apply(&view_record(
            Place { seq: 1, run },
            EventKind::RunStarted,
            &room,
            data,
        ))
        .unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    let summary = city.runs.iter().find(|r| r.run == run).unwrap();
    assert_eq!(summary.addr, Some(room));
    assert_eq!(summary.started, Some(kernel::TimeMs::new(1_000)));
}

/// A city that has run thousands of times used to put every run it ever
/// held on the wire at each asking (sprawling-SPEC section 8-90): 8,000
/// runs made a 1.37 MB city view, fetched again after every record.
#[test]
fn a_city_of_eight_thousand_runs_answers_in_a_bounded_view() {
    const RUNS: u64 = 8_000;
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run_of = |i: u64| RunId::from_bytes(u128::from(i + 1).to_be_bytes());
    for i in 0..RUNS {
        let mut opened = serde_json::Map::new();
        opened.insert("task".to_owned(), serde_json::Value::from("a task"));
        opened.insert("goal".to_owned(), serde_json::Value::from("a goal"));
        let mut closed = serde_json::Map::new();
        closed.insert("completion".to_owned(), serde_json::Value::from("done"));
        let (start, end) = (2 * i + 1, 2 * i + 2);
        views
            .apply(&view_record(
                Place {
                    seq: start,
                    run: run_of(i),
                },
                EventKind::RunStarted,
                &room,
                opened,
            ))
            .unwrap();
        views
            .apply(&view_record(
                Place {
                    seq: end,
                    run: run_of(i),
                },
                EventKind::RunFrozen,
                &room,
                closed,
            ))
            .unwrap();
    }

    let answer = views.answer(&channels::Query::CityView);
    let bytes = serde_json::to_vec(&answer).unwrap().len();
    let channels::Answer::City(city) = answer else {
        panic!("CityView answers with a city");
    };
    let recent = u64::try_from(memory::RECENT_FROZEN).unwrap();
    let listed: Vec<RunId> = city.runs.iter().map(|r| r.run).collect();
    let newest: Vec<RunId> = (RUNS - recent..RUNS).map(run_of).collect();
    assert!(bytes <= 16 * 1024, "city_view is {bytes} bytes");
    assert_eq!((listed, city.active, city.frozen), (newest, 0, RUNS));
}

/// A cost view used to name every run a city ever billed (sprawling-SPEC
/// section 8-90); it names every active run and the few billed most,
/// while `total` still sums them all.
#[test]
fn a_cost_view_of_eight_thousand_billed_runs_names_the_top_few() {
    const RUNS: u64 = 8_000;
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run_of = |i: u64| RunId::from_bytes(u128::from(i + 1).to_be_bytes());
    let billed = |micros: u64| {
        let mut data = serde_json::Map::new();
        data.insert(
            "billed_usd_micros".to_owned(),
            serde_json::Value::from(micros),
        );
        data
    };
    let mut seq = 0;
    let mut fold = |run: RunId, kind: EventKind, data| {
        seq += 1;
        views
            .apply(&view_record(Place { seq, run }, kind, &room, data))
            .unwrap();
    };
    for i in 0..=RUNS {
        fold(run_of(i), EventKind::RunStarted, serde_json::Map::new());
        let micros = if i == RUNS { 1 } else { i + 1 };
        fold(run_of(i), EventKind::ModelReturned, billed(micros));
        if i < RUNS {
            fold(run_of(i), EventKind::RunFrozen, serde_json::Map::new());
        }
    }

    let channels::Answer::Cost(cost) = views.answer(&channels::Query::CostView) else {
        panic!("CostView answers with a cost");
    };
    let top = u64::try_from(super::answering::TOP_BILLED).unwrap();
    let mut named: Vec<(String, u64)> = (RUNS - top..=RUNS)
        .map(|i| (run_of(i).to_string(), if i == RUNS { 1 } else { i + 1 }))
        .collect();
    named.sort();
    let listed: Vec<(String, u64)> = cost
        .by_run
        .iter()
        .map(|(run, usd)| (run.clone(), usd.get()))
        .collect();
    assert_eq!(
        (listed, cost.total.get()),
        (named, RUNS * (RUNS + 1) / 2 + 1)
    );
}

/// The hot view keeps only the recent frozen few (memory-SPEC section
/// 8-5); a run pushed out of it is still a run the city holds, so its
/// run view is answered from the Ledger instead of reading as unknown.
#[test]
fn an_evicted_run_still_answers_its_run_view() {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = crate::assembly::init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let run_of = |i: u64| RunId::from_bytes(u128::from(i + 1).to_be_bytes());
    let mut ledger = memory::JsonlLedger::open(&report.ledger_dir, kernel::TimeMs::new(9))
        .unwrap()
        .0;
    let recent = u64::try_from(memory::RECENT_FROZEN).unwrap();
    for i in 0..=recent {
        let opened = serde_json::json!({ "task": "a task", "goal": "a goal" });
        let closed = serde_json::json!({ "completion": "done" });
        for (kind, data) in [
            (EventKind::RunStarted, opened),
            (EventKind::RunFrozen, closed),
        ] {
            ledger
                .append(kernel::EventDraft {
                    run: run_of(i),
                    t: kernel::TimeMs::new(1_000 + i),
                    who: "city".to_owned(),
                    addr: Some(room.clone()),
                    kind,
                    data: kernel::Payload::new(data.as_object().unwrap().clone()).unwrap(),
                    ig: false,
                })
                .unwrap();
        }
    }
    drop(ledger);

    let mut views = Views::rebuild(&report.ledger_dir).unwrap();
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    assert!(city.runs.iter().all(|r| r.run != run_of(0)));
    let channels::Answer::Run(Some(summary)) =
        views.answer(&channels::Query::RunView { run: run_of(0) })
    else {
        panic!("an evicted run is still a run the city holds");
    };
    assert_eq!(
        (
            summary.frozen,
            summary.last_kind,
            summary.addr,
            summary.started
        ),
        (
            true,
            EventKind::RunFrozen,
            Some(room),
            Some(kernel::TimeMs::new(1_000))
        )
    );
}

/// A city of `RECENT_FROZEN` + 1 runs, each started, billed `1_000 + i`
/// micro-dollars and frozen, written to the Ledger and folded back, so
/// the first of them is out of the hot view.
fn a_city_whose_first_billed_run_was_evicted(root: &std::path::Path) -> Views {
    use kernel::Ledger;
    let report = crate::assembly::init_city(root).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let mut ledger = memory::JsonlLedger::open(&report.ledger_dir, kernel::TimeMs::new(9))
        .unwrap()
        .0;
    let recent = u64::try_from(memory::RECENT_FROZEN).unwrap();
    for i in 0..=recent {
        for (kind, data) in [
            (
                EventKind::RunStarted,
                serde_json::json!({ "task": "t", "goal": "g" }),
            ),
            (
                EventKind::ModelReturned,
                serde_json::json!({ "billed_usd_micros": 1_000 + i }),
            ),
            (
                EventKind::RunFrozen,
                serde_json::json!({ "completion": "done" }),
            ),
        ] {
            ledger
                .append(kernel::EventDraft {
                    run: billed_run(i),
                    t: kernel::TimeMs::new(1_000 + i),
                    who: "city".to_owned(),
                    addr: Some(room.clone()),
                    kind,
                    data: kernel::Payload::new(data.as_object().unwrap().clone()).unwrap(),
                    ig: false,
                })
                .unwrap();
        }
    }
    drop(ledger);
    Views::rebuild(&report.ledger_dir).unwrap()
}

fn billed_run(i: u64) -> RunId {
    RunId::from_bytes(u128::from(i + 1).to_be_bytes())
}

/// The building directory asks what an old run cost by name; a run the
/// city never saw spent nothing, which is an answer, not a gap.
#[test]
fn an_evicted_run_still_answers_what_it_cost() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = a_city_whose_first_billed_run_was_evicted(dir.path());
    let never = RunId::from_bytes([0xee; 16]);
    let asked = channels::Query::RunCosts {
        runs: vec![billed_run(0), never],
    };
    assert_eq!(
        views.answer(&asked),
        channels::Answer::RunCosts(channels::RunCostsAnswer {
            asked: vec![billed_run(0), never],
            runs: vec![
                (billed_run(0), kernel::UsdMicros::new(1_000)),
                (never, kernel::UsdMicros::default()),
            ],
        })
    );
}

/// The attribution kept a row for every run the city ever billed; once
/// the hot view evicts a run its row goes too, the money stays in the
/// total, and the cold side still answers what the run cost.
#[test]
fn the_attribution_holds_only_the_runs_the_hot_view_holds() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = a_city_whose_first_billed_run_was_evicted(dir.path());
    let recent = u64::try_from(memory::RECENT_FROZEN).unwrap();
    let billed: u64 = (0..=recent).map(|i| 1_000 + i).sum();
    let report = views.attribution.report();
    assert_eq!(
        (
            views.attribution.billed_to(&billed_run(0)),
            report.by_run.len(),
            report.total
        ),
        (None, memory::RECENT_FROZEN, kernel::UsdMicros::new(billed))
    );
    let asked = channels::Query::RunCosts {
        runs: vec![billed_run(0)],
    };
    assert_eq!(
        views.answer(&asked),
        channels::Answer::RunCosts(channels::RunCostsAnswer {
            asked: vec![billed_run(0)],
            runs: vec![(billed_run(0), kernel::UsdMicros::new(1_000))],
        })
    );
}
