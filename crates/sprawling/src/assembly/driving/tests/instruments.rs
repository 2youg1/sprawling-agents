// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Two instruments over the accounting loop the city runs
//! (sprawling-SPEC.md 8-83): a relay round trip, and the gap a second
//! dispatch leaves in a run that is already going.
//!
//! Both drive `serving::attending::attend` on a thread of its own, send
//! work in through `CommandDesk::post`, and talk to the loopback
//! provider. Both are ignored by `just check`, because they read the
//! wall clock and take seconds; `just bench` runs them and prints one
//! reading line each.

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

use crate::assembly::fixture::*;
use crate::assembly::*;
use crate::serving::CommandDesk;
use crate::serving::attending::attend;

/// Appends timed per store: enough for a stable middle, few enough that
/// the disk store stays inside a few seconds at a barrier each.
const APPENDS: usize = 200;
/// What the digest model takes to name a room in the gap scenario,
/// which is about what a remote model takes to answer a short request.
const NAMING: Duration = Duration::from_secs(3);
/// The status turns run A takes before it answers.
const TURNS: usize = 30;
/// Which of A's model calls sends B in.
const B_ARRIVES_AT_CALL: usize = 5;
/// A phrase only run A's requests carry.
const A_TASK: &str = "tally the east kiln";
/// The phrase only the naming request carries.
const NAMING_PHRASE: &str = "Name this piece of work";
/// The longest the scenario may take before it is reported as stuck.
const WITHIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
enum Store {
    Disk,
    Memory,
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_relay_round_trip() {
    for store in [Store::Disk, Store::Memory] {
        let taken = relay_round_trips(store);
        report(
            "instrument_relay_round_trip",
            &format!("store={}", store_name(store)),
            taken,
        );
    }
    report(
        "instrument_relay_round_trip",
        "store=memory crossing=none",
        own_appends(),
    );
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_dispatch_gap() {
    let dir = tempfile::tempdir().unwrap();
    raise_lab(dir.path());
    let desk = Arc::new(CommandDesk::new());
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
                    channels::Reply::to(|refused| panic!("run B was refused: {refused}")),
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
        vec![completion("benchroom", None), completion("glazed", None)],
        pace,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    // B is sent to a building with no room, so a digest model names it.
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"digest"),
        })
        .unwrap();
    let attending = attending(worker, &desk);
    desk.post(dispatch("lab/east", A_TASK, b"a"), nowhere());
    let lines = until_frozen(dir.path(), 2);
    desk.close();
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
            .any(|line| line["kind"] == "run_started" && line["addr"] == "lab/benchroom"),
        "run B was named and started, so the scenario happened"
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
}

/// `APPENDS` relay appends, each timed, while a run is driving and
/// held at its first model call - the shape the loop is in whenever a
/// lane is out.
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
                gateway::Custodian::in_memory(),
                runtime::diagnostics::Diagnostics::off(),
                in_memory_ledger(&ledger_dir(dir.path())),
            )
            .unwrap(),
            &base_url,
            "m-local",
        )
        .unwrap(),
    };
    let mut relay = worker.measuring_relay();
    let desk = Arc::new(CommandDesk::new());
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
    desk.close();
    attending.join().unwrap();
    drop(provider);
    taken
}

/// The same appends into a ledger on the in-memory store, with no
/// crossing: what the accounting thread's own half of a round trip
/// costs.
fn own_appends() -> Vec<Duration> {
    let dir = tempfile::tempdir().unwrap();
    let mut ledger = in_memory_ledger(dir.path());
    (0..APPENDS)
        .map(|_| {
            let draft = marker();
            let started = Instant::now();
            kernel::Ledger::append(&mut ledger, draft).unwrap();
            started.elapsed()
        })
        .collect()
}

fn in_memory_ledger(dir: &std::path::Path) -> memory::JsonlLedger {
    let fs = memory::FaultFs::new(memory::FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: memory::TornTail::None,
    });
    memory::JsonlLedger::open_faulty(fs, dir, now_ms().unwrap())
        .unwrap()
        .0
}

/// A city with one building and one room in it, under ordinary rules.
fn raise_lab(root: &std::path::Path) {
    init_city(root).unwrap();
    std::fs::create_dir_all(root.join("lab").join("east")).unwrap();
    lay_rules(root, "lab", &ordinary_rules(""));
}

/// The accounting loop on a thread of its own, as `spawn_worker` runs it.
fn attending(worker: RunWorker, desk: &Arc<CommandDesk>) -> std::thread::JoinHandle<()> {
    let desk = Arc::clone(desk);
    std::thread::spawn(move || {
        let mut worker = worker;
        attend(&mut worker, &desk);
    })
}

fn dispatch(addr: &str, task: &str, key: &[u8]) -> channels::Command {
    channels::Command::Dispatch {
        addr: Address::parse(addr).unwrap(),
        task: task.to_owned(),
        goal: format!("{task}, done"),
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        effort: None,
    }
}

fn nowhere() -> channels::Reply {
    channels::Reply::nowhere()
}

/// One record with nothing in it but its position.
fn marker() -> kernel::EventDraft {
    kernel::EventDraft {
        run: RunId::CITY,
        t: now_ms().unwrap(),
        who: "city".to_owned(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: kernel::Payload::empty(),
        ig: false,
    }
}

/// The history once `runs` runs have frozen, oldest first.
fn until_frozen(root: &std::path::Path, runs: usize) -> Vec<serde_json::Value> {
    let started = Instant::now();
    loop {
        let lines: Vec<serde_json::Value> = memory::read_raw_lines_at(&ledger_dir(root))
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

fn report(instrument: &str, fields: &str, mut taken: Vec<Duration>) {
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
}

fn machine() -> String {
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!(
        "machine={}-{}, {cores} core(s)",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
