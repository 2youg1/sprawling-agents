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
    clippy::disallowed_methods,
    reason = "test code: an instrument reads the wall clock"
)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::worker::CommandDesk;
use crate::worker::attend::attend;
use crate::worker::fixture::*;
use crate::worker::*;

const APPENDS: usize = 200;

const NAMING: Duration = Duration::from_secs(3);

const TURNS: usize = 30;

const B_ARRIVES_AT_CALL: usize = 5;

const A_TASK: &str = "tally the east kiln";

const NAMING_PHRASE: &str = "Name this piece of work";

const WITHIN: Duration = Duration::from_secs(60);

const ROUND_TRIP_P50: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, Copy)]
enum Store {
    Disk,
    Memory,
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_relay_round_trip() {
    report(
        "instrument_relay_round_trip",
        "store=memory crossing=none",
        own_appends(),
    );
    let middles: Vec<(Store, Duration)> = [Store::Disk, Store::Memory]
        .into_iter()
        .map(|store| {
            let p50 = report(
                "instrument_relay_round_trip",
                &format!("store={}", store_name(store)),
                relay_round_trips(store),
            );
            (store, p50)
        })
        .collect();
    for (store, p50) in middles {
        if matches!(store, Store::Disk) {
            continue;
        }
        assert!(
            p50 <= ROUND_TRIP_P50,
            "a relay round trip on store={} took {p50:?} at its middle, over {ROUND_TRIP_P50:?}",
            store_name(store)
        );
    }
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_dispatch_gap() {
    let dir = tempfile::tempdir().unwrap();
    raise_lab(dir.path());
    let desk = Arc::new(CommandDesk::default());
    let a_calls = AtomicUsize::new(0);
    let pace: Pace = {
        let desk = Arc::clone(&desk);
        Arc::new(move |request: &str| {
            if request.contains(NAMING_PHRASE) {
                std::thread::sleep(NAMING);
            } else if request.contains(A_TASK)
                && a_calls.fetch_add(1, Ordering::SeqCst) + 1 == B_ARRIVES_AT_CALL
            {
                desk.post(
                    dispatch("lab", "glaze the west kiln", b"b"),
                    wire::Reply::to(|refused| panic!("run B was refused: {refused}")),
                );
            }
        })
    };
    let a_replies = (0..TURNS)
        .map(|turn| {
            completion_with(
                "looking",
                "status",
                &format!("tu_{turn}"),
                serde_json::json!({}),
            )
        })
        .chain([completion("tallied", None)])
        .collect();
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        vec![(A_TASK, a_replies)],
        vec![completion("glazed", None)],
        pace,
    );
    let worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let attending = attending(worker, &desk);
    desk.post(dispatch("lab/east", A_TASK, b"a"), nowhere());
    let lines = until_frozen(dir.path(), 2);
    desk.close(Closing::Chosen {
        by: crate::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();
    drop(provider);

    let a_run = lines
        .iter()
        .find(|line| line["kind"] == "run_started" && line["addr"] == "lab/east")
        .map(|line| line["run"].clone())
        .expect("run A started");
    assert!(
        lines
            .iter()
            .any(|line| line["kind"] == "run_started" && line["addr"] == "lab/glaze-the-west-kiln"),
        "run B was named by rule and started, so the scenario happened"
    );
    let stamps: Vec<u64> = lines
        .iter()
        .filter(|line| line["run"] == a_run)
        .map(|line| line["t"].as_u64().unwrap())
        .collect();
    let mut gaps: Vec<u64> = stamps.windows(2).map(|pair| pair[1] - pair[0]).collect();
    gaps.sort_unstable();
    println!(
        "instrument_dispatch_gap samples={} max_ms={} median_ms={} {}",
        gaps.len(),
        gaps.last().copied().unwrap_or(0),
        gaps.get(gaps.len() / 2).copied().unwrap_or(0),
        machine()
    );

    let widest = gaps.last().copied().unwrap_or(0);
    let naming = u64::try_from(NAMING.as_millis()).unwrap();
    assert!(
        widest < naming,
        "run A stood still for {widest} ms while run B was dispatched, as long as the {naming} ms naming call"
    );
}

fn relay_round_trips(store: Store) -> Vec<Duration> {
    let dir = tempfile::tempdir().unwrap();
    raise_lab(dir.path());
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
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![completion("done", None)],
        pace,
    );
    let worker = match store {
        Store::Disk => worker_with_provider(dir.path(), &base_url, "m-local").unwrap(),
        Store::Memory => attach_provider(
            RunWorker::over(
                dir.path(),
                runtime::diagnostics::Diagnostics::off(),
                crate::worker::fixture::hands(),
                in_memory_ledger(&kernel::layout::CityLayout::new(dir.path()).ledger()),
            )
            .unwrap(),
            &base_url,
            "m-local",
        )
        .unwrap(),
    };
    let mut relay = worker.measuring_relay();
    let desk = Arc::new(CommandDesk::default());
    let attending = attending(worker, &desk);
    desk.post(dispatch("lab/east", "fire the east kiln", b"a"), nowhere());
    arrived.recv_timeout(WITHIN).unwrap();
    let taken = (0..APPENDS)
        .map(|_| {
            let draft = marker();
            let started = Instant::now();
            kernel::Ledger::append(&mut relay, draft).unwrap();
            started.elapsed()
        })
        .collect();
    release.send(()).unwrap();
    desk.close(Closing::Chosen {
        by: crate::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();
    drop(provider);
    taken
}

fn own_appends() -> Vec<Duration> {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = in_memory_ledger(dir.path());
    (0..APPENDS)
        .map(|_| {
            let draft = marker();
            let started = Instant::now();
            kernel::Ledger::append(&mut ledger, draft).unwrap();
            started.elapsed()
        })
        .collect()
}

fn in_memory_ledger(dir: &std::path::Path) -> (storage::JsonlLedger, storage::OpenReport) {
    let fs = storage::FaultFs::new(storage::FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: storage::TornTail::None,
    });
    storage::JsonlLedger::open_faulty(
        fs,
        dir,
        crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
    )
    .unwrap()
}

fn raise_lab(root: &std::path::Path) {
    crate::worker::fixture::init_city(root).unwrap();
    std::fs::create_dir_all(root.join("lab").join("east")).unwrap();
    lay_rules(root, "lab", &ordinary_rules(""));
}

pub(super) fn attending(worker: RunWorker, desk: &Arc<CommandDesk>) -> std::thread::JoinHandle<()> {
    let desk = Arc::clone(desk);
    std::thread::spawn(move || {
        let mut worker = worker;
        attend(&mut worker, &desk);
    })
}

pub(super) fn dispatch(addr: &str, task: &str, key: &[u8]) -> wire::Command {
    wire::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: task.to_owned(),
        goal: format!("{task}, done"),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        resume: None,
        images: Vec::new(),
        effort: None,
        model: None,
    }
}

fn nowhere() -> wire::Reply {
    wire::Reply::nowhere()
}

pub(super) fn marker() -> kernel::EventDraft {
    kernel::EventDraft {
        run: RunId::CITY,
        t: crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
        who: "city".to_owned(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: kernel::Payload::empty(),
        ig: false,
    }
}

fn until_frozen(root: &std::path::Path, runs: usize) -> Vec<serde_json::Value> {
    let started = Instant::now();
    loop {
        let lines: Vec<serde_json::Value> =
            storage::read_raw_lines_at(&kernel::layout::CityLayout::new(root).ledger())
                .unwrap_or_default()
                .iter()
                .filter_map(|line| serde_json::from_slice(line).ok())
                .collect();
        if lines
            .iter()
            .filter(|line| line["kind"] == "run_frozen")
            .count()
            >= runs
        {
            return lines;
        }
        assert!(
            started.elapsed() < WITHIN,
            "{runs} runs never froze: {:?}",
            lines
                .iter()
                .map(|line| format!("{} {} {}", line["kind"], line["addr"], line["run"]))
                .collect::<Vec<_>>()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn store_name(store: Store) -> &'static str {
    match store {
        Store::Disk => "disk",
        Store::Memory => "memory",
    }
}

fn report(instrument: &str, fields: &str, mut taken: Vec<Duration>) -> Duration {
    taken.sort_unstable();
    let floor = taken.first().copied().unwrap_or_default();
    let p50 = taken
        .get(taken.len().div_ceil(2).saturating_sub(1))
        .copied()
        .unwrap_or_default();
    println!(
        "{instrument} {fields} samples={} floor_us={} p50_us={} {}",
        taken.len(),
        floor.as_micros(),
        p50.as_micros(),
        machine()
    );
    p50
}

pub(super) fn machine() -> String {
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!(
        "machine={}-{}, {cores} core(s)",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
