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
use crate::worker::fixture::*;
use crate::worker::*;

/// A dispatch as the desk delivers one: a person's work, sent to a room
/// that already exists.
pub(super) fn asked(addr: &str) -> Assignment {
    Assignment {
        addr: Address::parse(addr).unwrap(),
        session: None,
        effort: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        parent: None,
        succession: None,
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::Person,
        origin: None,
    }
}

/// Every line of the city's history, parsed, oldest first.
pub(super) fn history(ledger_dir: &std::path::Path) -> Vec<serde_json::Value> {
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
/// The assertion that bites is the count of lanes in the air before the
/// loop that lands them runs: driven one at a time, the second waits.
/// The history cannot say it, because each run prepares in its own lane
/// and how its first line interleaves with the other's freeze is a race.
#[test]
fn two_dispatches_from_the_desk_drive_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("west")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
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
            Owing::asked(wire::Reply::nowhere()),
        )
        .unwrap();
    worker
        .dispatch_into_lane(
            asked("lab/west"),
            "fire the west kiln".to_owned(),
            "the west kiln is fired".to_owned(),
            Owing::asked(wire::Reply::nowhere()),
        )
        .unwrap();
    assert_eq!(
        worker.flight.in_flight(),
        2,
        "both drives are in the air before either is landed"
    );
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
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
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
            Owing::asked(wire::Reply::nowhere()),
        )
        .unwrap();
    assert!(worker.driving(), "a lane is going");
    worker.land_the_rest().unwrap();
    assert!(!worker.driving(), "the closing city waited for it");
    worker.close_city(&Closing::Chosen).unwrap();
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

/// **A Cancel posted while a lane drives reaches that lane.**
///
/// The accounting thread reads the desk on every wake, and a lane reads
/// it only at its safe points, so the thread nearly always saw the
/// Cancel first and refused it as though no run answered to it. The
/// bite is the first assertion: a refusal reached the person while the
/// run it named was still in its model call.
#[test]
fn a_cancel_posted_while_a_lane_drives_stops_that_run() {
    use std::sync::{Arc, Mutex, mpsc};
    const WITHIN: std::time::Duration = std::time::Duration::from_secs(60);
    const LOOK: std::time::Duration = std::time::Duration::from_millis(5);
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("east")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let (arrived_tx, arrived) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let held = Mutex::new((Some(arrived_tx), released));
    let pace: Pace = Arc::new(move |request: &str| {
        if !request.starts_with("POST ") {
            return;
        }
        let mut held = held.lock().unwrap();
        if let Some(arrived) = held.0.take() {
            arrived.send(()).unwrap();
            held.1.recv().unwrap();
        }
    });
    let looking = completion_with("looking", "status", "tu_0", serde_json::json!({}));
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![looking, completion("done", None)],
        pace,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let desk = Arc::new(crate::worker::CommandDesk::default());
    let asking = Arc::clone(&desk);
    worker.serve(only_interrupts(Arc::new(move |run| {
        asking.interrupt_for(run)
    })));
    let attending = {
        let desk = Arc::clone(&desk);
        std::thread::spawn(move || crate::worker::attend::attend(&mut worker, &desk))
    };
    let key = |material: &[u8]| kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, material);
    desk.post(
        wire::Command::Dispatch {
            addr: Address::parse("lab/east").unwrap(),
            task: "fire the east kiln".to_owned(),
            goal: "the east kiln is fired".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: key(b"dispatch"),
            session: None,
            effort: None,
            model: None,
        },
        wire::Reply::nowhere(),
    );
    arrived.recv_timeout(WITHIN).unwrap();
    let run = history(&report.ledger_dir)
        .iter()
        .find(|line| line["kind"] == "run_started")
        .map(|line| RunId::parse(line["run"].as_str().unwrap()).unwrap())
        .expect("the run in its model call said it started");

    let heard = |into: &Arc<Mutex<Vec<AxError>>>| {
        let told = Arc::clone(into);
        wire::Reply::to(move |error| {
            told.lock().unwrap().push(error);
            wire::Delivered::ToThePeer
        })
    };
    let for_the_run = Arc::new(Mutex::new(Vec::new()));
    let for_nobody = Arc::new(Mutex::new(Vec::new()));
    desk.post(
        wire::Command::Cancel {
            run,
            idem: key(b"cancel the kiln"),
        },
        heard(&for_the_run),
    );
    // A Cancel no run answers to, posted behind it: once its refusal is
    // back, the accounting thread has looked at everything before it.
    desk.post(
        wire::Command::Cancel {
            run: RunId::CITY,
            idem: key(b"cancel nothing"),
        },
        heard(&for_nobody),
    );
    // Counted looks rather than a clock, which test code may not read:
    // each look sleeps `LOOK`, so the count bounds the wait at `WITHIN`.
    let looks = WITHIN.as_millis() / LOOK.as_millis();
    let mut looked = 0;
    while for_nobody.lock().unwrap().is_empty() {
        looked += 1;
        assert!(looked < looks, "the desk was never read");
        std::thread::sleep(LOOK);
    }
    assert_eq!(
        *for_the_run.lock().unwrap(),
        Vec::<AxError>::new(),
        "a Cancel for a run in a lane is that lane's to take, not a refusal"
    );

    release.send(()).unwrap();
    let mut looked = 0;
    while !history(&report.ledger_dir)
        .iter()
        .any(|line| line["kind"] == "run_frozen")
    {
        looked += 1;
        assert!(looked < looks, "the run never froze");
        std::thread::sleep(LOOK);
    }
    desk.close(Closing::Chosen);
    attending.join().unwrap();
    drop(provider);
    let run = run.to_string();
    let kinds: Vec<String> = history(&report.ledger_dir)
        .iter()
        .filter(|line| line["run"] == run.as_str())
        .map(|line| line["kind"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        kinds.iter().any(|kind| kind == "cancel_received"),
        "the lane took the Cancel at its next safe point: {kinds:?}"
    );
    assert!(
        !kinds.iter().any(|kind| kind == "tool_result"),
        "a cancelled run carries out no tool: {kinds:?}"
    );
}

/// **The lane count is one wall for every entrance.** Five pieces of
/// work sent at once drive four at a time: the fifth waits in the pool
/// rather than opening a thread to park on the provider's admission,
/// and starts when a lane comes home, so all five still freeze.
#[test]
fn work_past_the_lane_count_waits_for_a_lane() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let rooms = ["lab/a", "lab/b", "lab/c", "lab/d", "lab/e"];
    for room in rooms {
        std::fs::create_dir_all(dir.path().join(room)).unwrap();
    }
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_TWO_FREE_ROWS,
    )
    .unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        rooms.iter().map(|_| completion("done", None)).collect(),
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for room in rooms {
        worker
            .dispatch_into_lane(
                asked(room),
                "fire the kiln".to_owned(),
                "the kiln is fired".to_owned(),
                Owing::asked(wire::Reply::nowhere()),
            )
            .unwrap();
    }
    assert_eq!(
        worker.flight.in_flight(),
        crate::worker::pool::DRIVING_LANES,
        "no more runs drive at once than there are lanes"
    );
    worker.land_the_rest().unwrap();
    drop(provider);
    let frozen = history(&report.ledger_dir)
        .iter()
        .filter(|line| line["kind"] == "run_frozen")
        .count();
    assert_eq!(
        frozen,
        rooms.len(),
        "the run that waited for a lane ran too"
    );
}
