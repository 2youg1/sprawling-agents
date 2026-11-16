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

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::instruments::{attending, dispatch, machine, marker};
use crate::worker::CommandDesk;
use crate::worker::fixture::*;
use crate::worker::*;
use lines::{chosen, wait_line};

const RUN_COUNTS: [usize; 4] = [1, 4, 16, 64];

const TOOL_CALLS: usize = 4;

const P999_FLOOR: usize = 1_000;

mod lines;

const MEASURED_LATENCY_MS: [u64; 10] =
    [991, 1462, 1901, 2753, 3281, 4605, 6409, 8711, 14898, 29415];

const WITHIN: Duration = Duration::from_secs(30 * 60);

const RELAY_BEAT: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Latency {
    Zero,
    Fixed50,
    Measured,
}

impl Latency {
    const ALL: [Latency; 3] = [Latency::Zero, Latency::Fixed50, Latency::Measured];

    fn name(self) -> &'static str {
        match self {
            Latency::Zero => "zero",
            Latency::Fixed50 => "fixed50",
            Latency::Measured => "measured",
        }
    }

    fn hold(self, call: usize) -> Duration {
        match self {
            Latency::Zero => Duration::ZERO,
            Latency::Fixed50 => Duration::from_millis(50),
            Latency::Measured => {
                Duration::from_millis(MEASURED_LATENCY_MS[call % MEASURED_LATENCY_MS.len()])
            }
        }
    }
}

struct Taken {
    lines: Vec<serde_json::Value>,
    relay: Vec<Duration>,

    relay_queue: Vec<u64>,

    lane_wait: Vec<u64>,
    idle_us: u64,
    posted_ms: u64,
    wall: Duration,
}

#[test]
fn throughput_counts() {
    let runs = 4;
    let taken = scenario(runs, Latency::Zero);
    let count = |kind: &str| {
        taken
            .lines
            .iter()
            .filter(|line| line["kind"] == kind)
            .count()
    };
    assert_eq!(
        (
            count("run_frozen"),
            count("tool_called"),
            count("tool_result")
        ),
        (runs, runs * TOOL_CALLS, runs * TOOL_CALLS),
        "the scenario's counts depend on its script alone"
    );
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_throughput() {
    let idle = relay_idle();
    for latency in chosen("SPRAWLING_TP1_LATENCY", &Latency::ALL, |arm: &Latency| {
        arm.name().to_owned()
    }) {
        for runs in chosen("SPRAWLING_TP1_RUNS", &RUN_COUNTS, usize::to_string) {
            let taken = scenario(runs, latency);
            let head = format!("n_runs={runs} latency={}", latency.name());
            println!("{}", throughput_line(&head, &taken));
            for (wait, samples) in waits(&taken, &idle) {
                println!("{}", wait_line(&head, wait, samples));
            }
        }
    }
}

fn scenario(runs: usize, latency: Latency) -> Taken {
    let dir = tempfile::tempdir().unwrap();
    raise_rooms(dir.path(), runs);
    let calls = Arc::new(AtomicUsize::new(0));
    let pace: Pace = Arc::new(move |request: &str| {
        if request.starts_with("POST ") {
            std::thread::sleep(latency.hold(calls.fetch_add(1, Ordering::SeqCst)));
        }
    });
    let routes: Vec<(String, Vec<String>)> =
        (0..runs).map(|room| (task(room), script(room))).collect();
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        routes
            .iter()
            .map(|(key, replies)| (key.as_str(), replies.clone()))
            .collect(),
        vec![completion("done", None)],
        pace,
    );
    let worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let relay = worker.measuring_relay();
    let health = worker.health();
    let desk = Arc::new(CommandDesk::default());
    let attending = attending(worker, &desk);
    let before = history(dir.path()).len();
    let queue_before = health.relay_queue_us().len();
    let lane_before = health.lane_wait_us().len();
    let idle_before = health.idle_us();
    let posted_ms = crate::Clock::now(&WallClock).unwrap().value();
    let started = Instant::now();
    for room in 0..runs {
        desk.post(
            dispatch(
                &format!("lab/r{room:02}"),
                &task(room),
                task(room).as_bytes(),
            ),
            wire::Reply::nowhere(),
        );
    }
    let done = Arc::new(AtomicBool::new(false));
    let sampler = sample_relay(relay, Arc::clone(&done));
    let lines = until_frozen(dir.path(), runs);
    let wall = started.elapsed();
    let idle_us = health.idle_us().saturating_sub(idle_before);
    let relay_queue = health
        .relay_queue_us()
        .into_iter()
        .skip(queue_before)
        .collect();
    let lane_wait = health
        .lane_wait_us()
        .into_iter()
        .skip(lane_before)
        .collect();
    done.store(true, Ordering::SeqCst);
    let relay = sampler.join().unwrap();
    desk.close(Closing::Chosen {
        by: crate::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();
    drop(provider);
    Taken {
        lines: lines.into_iter().skip(before).collect(),
        relay,
        relay_queue,
        lane_wait,
        idle_us,
        posted_ms,
        wall,
    }
}

fn task(room: usize) -> String {
    format!("tally kiln k{room:02}x")
}

fn script(room: usize) -> Vec<String> {
    (0..TOOL_CALLS)
        .map(|call| {
            let id = format!("tu_{room}_{call}");
            if call % 2 == 0 {
                completion_with("looking", "status", &id, serde_json::json!({}))
            } else {
                completion("noting", Some((&id, &format!("note-{room}-{call}.txt"))))
            }
        })
        .chain([completion("tallied", None)])
        .collect()
}

fn raise_rooms(root: &std::path::Path, runs: usize) {
    init_city(root).unwrap();
    for room in 0..runs {
        std::fs::create_dir_all(root.join("lab").join(format!("r{room:02}"))).unwrap();
    }
    lay_rules(root, "lab", &ordinary_rules(""));
}

fn sample_relay(mut relay: Relay, done: Arc<AtomicBool>) -> std::thread::JoinHandle<Vec<Duration>> {
    std::thread::spawn(move || {
        let mut taken = Vec::new();
        while !done.load(Ordering::SeqCst) {
            let draft = marker();
            let started = Instant::now();
            kernel::Ledger::append(&mut relay, draft).unwrap();
            taken.push(started.elapsed());
            std::thread::sleep(RELAY_BEAT);
        }
        taken
    })
}

fn relay_idle() -> Vec<Duration> {
    let dir = tempfile::tempdir().unwrap();
    raise_rooms(dir.path(), 1);
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![completion("done", None)],
        Arc::new(|_: &str| {}),
    );
    let worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let relay = worker.measuring_relay();
    let desk = Arc::new(CommandDesk::default());
    let attending = attending(worker, &desk);
    let done = Arc::new(AtomicBool::new(false));
    let sampler = sample_relay(relay, Arc::clone(&done));
    std::thread::sleep(Duration::from_secs(4));
    done.store(true, Ordering::SeqCst);
    let taken = sampler.join().unwrap();
    desk.close(Closing::Chosen {
        by: crate::worker::ClosedBy::Console,
        mode: wire::CloseMode::Drain,
    });
    attending.join().unwrap();
    drop(provider);
    taken
}

fn history(root: &std::path::Path) -> Vec<serde_json::Value> {
    storage::read_raw_lines_at(&kernel::layout::CityLayout::new(root).ledger())
        .unwrap()
        .iter()
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

fn until_frozen(root: &std::path::Path, runs: usize) -> Vec<serde_json::Value> {
    let started = Instant::now();
    loop {
        let lines = history(root);
        if lines
            .iter()
            .filter(|line| line["kind"] == "run_frozen")
            .count()
            >= runs
        {
            return lines;
        }
        assert!(started.elapsed() < WITHIN, "{runs} runs never froze");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn throughput_line(head: &str, taken: &Taken) -> String {
    let count = |kind: &str| {
        taken
            .lines
            .iter()
            .filter(|line| line["kind"] == kind)
            .count()
    };
    let runs = count("run_frozen");

    let records = taken.lines.len() - count("city_initialized");
    let tool_calls = count("tool_called");
    let wall_us = taken.wall.as_micros().max(1);
    let per_s = |n: usize| {
        let milli = u128::try_from(n).unwrap() * 1_000_000_000 / wall_us;
        format!("{}.{:03}", milli / 1000, milli % 1000)
    };
    format!(
        "throughput {head} tool_calls_per_run={TOOL_CALLS} runs={runs} records={records} tool_calls={tool_calls} wall_us={wall_us} runs_per_s={} records_per_s={} tool_calls_per_s={} ledger_thread_idle_us={} ledger_thread_busy_permille={} {}",
        per_s(runs),
        per_s(records),
        per_s(tool_calls),
        taken.idle_us,
        wall_us.saturating_sub(u128::from(taken.idle_us)) * 1000 / wall_us,
        machine()
    )
}

fn waits(taken: &Taken, idle: &[Duration]) -> Vec<(&'static str, Vec<u64>)> {
    let micros = |samples: &[Duration]| {
        samples
            .iter()
            .map(|taken| u64::try_from(taken.as_micros()).unwrap())
            .collect()
    };
    let mut started: BTreeMap<String, u64> = BTreeMap::new();
    let mut first_call: BTreeMap<String, u64> = BTreeMap::new();
    for line in &taken.lines {
        let run = line["run"].to_string();
        let t = line["t"].as_u64().unwrap_or(0);
        if line["kind"] == "run_started" {
            started.insert(run, t);
        } else if line["kind"] == "model_called" {
            first_call.entry(run).or_insert(t);
        }
    }
    let lane = started
        .values()
        .map(|t| t.saturating_sub(taken.posted_ms) * 1000)
        .collect();
    let to_first_call = started
        .iter()
        .filter_map(|(run, t)| {
            first_call
                .get(run)
                .map(|call| call.saturating_sub(*t) * 1000)
        })
        .collect();
    vec![
        ("lane", lane),
        ("first_call", to_first_call),
        ("relay_under_load", micros(&taken.relay)),
        ("relay_queue", taken.relay_queue.clone()),
        ("lane_pure", taken.lane_wait.clone()),
        ("relay_idle", micros(idle)),
    ]
}
