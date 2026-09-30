// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How far the view fold falls behind a burst of records
//! (sprawling-SPEC.md 8-99).
//!
//! The instrument drives the fold thread the city runs, `spawn_folding`,
//! with a writer sending a burst through `observer` and a reader asking
//! the published views between batches. It is ignored by `just check`,
//! because it reads the wall clock; `just bench` runs it and prints one
//! reading line. It asserts nothing about time, since the wall clock
//! differs by machine.
//!
//! The burst is signal lines as `collab` writes them, the same shape the
//! bench Main's `draft` gives its fold scenario; this second spelling of
//! that shape exists only for this instrument, which cannot reach a bin
//! of another package.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::disallowed_methods,
    reason = "test code: an instrument reads the wall clock"
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kernel::{Address, EventDraft, EventKind, EventRecord, Payload, RunId, TimeMs};

use crate::assembly::init_city;
use crate::serving::folding::{Broadcast, Copies, spawn_folding};
use crate::serving::standing::monotonic_now;
use crate::views::{Published, Views, answer_outside_the_lock};
use accounting::person::CorePriority;

/// Records sent in the burst, beside the lines `init` writes.
const BURST: u64 = 2_000;
/// The longest the fold may take to broadcast the whole burst before the
/// instrument reports it stuck.
const WITHIN: Duration = Duration::from_secs(60);

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_view_backlog() {
    let dir = tempfile::tempdir().unwrap();
    let records = burst_over(dir.path());
    let unfolded = Views::new(dir.path());
    let spare = unfolded.unfolded_twin();
    let views = Arc::new(Published::new(unfolded));
    let (to_clients, mut heard) = tokio::sync::broadcast::channel(records.len() + 64);
    let broadcast = Broadcast {
        to_clients,
        head: Arc::new(wire::LedgerHead::default()),
    };
    let copies = Copies {
        published: Arc::clone(&views),
        spare,
    };
    let folding = spawn_folding(copies, broadcast, CorePriority::Raised, monotonic_now).unwrap();

    let reading = Arc::new(AtomicBool::new(true));
    let reader = {
        let (views, reading) = (Arc::clone(&views), Arc::clone(&reading));
        std::thread::spawn(move || {
            let mut asked = 0u64;
            while reading.load(Ordering::Relaxed) {
                let (_as_of, answered) = answer_outside_the_lock(&views, &wire::Query::CityView);
                std::hint::black_box(answered.unwrap());
                asked += 1;
            }
            asked
        })
    };
    let sent = Arc::new(AtomicUsize::new(0));
    let mut observer = folding.observer;
    let writer = {
        let (records, sent) = (records.clone(), Arc::clone(&sent));
        std::thread::spawn(move || {
            let mut sent_at = Vec::with_capacity(records.len());
            for record in &records {
                sent_at.push(Instant::now());
                observer(record);
                sent.fetch_add(1, Ordering::SeqCst);
            }
            (sent_at, observer)
        })
    };

    let started = Instant::now();
    let mut heard_at = Vec::with_capacity(records.len());
    let mut max_backlog = 0usize;
    while heard_at.len() < records.len() {
        match heard.try_recv() {
            Ok(_) => {
                heard_at.push(Instant::now());
                let behind = sent.load(Ordering::SeqCst).saturating_sub(heard_at.len());
                max_backlog = max_backlog.max(behind);
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                assert!(
                    started.elapsed() < WITHIN,
                    "the fold broadcast {} of {} records",
                    heard_at.len(),
                    records.len()
                );
                std::thread::yield_now();
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(missed)) => {
                panic!("the instrument fell {missed} records behind its own broadcast")
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                panic!("the fold ended before it broadcast the burst")
            }
        }
    }
    let (sent_at, observer) = writer.join().unwrap();
    reading.store(false, Ordering::Relaxed);
    let asked = reader.join().unwrap();
    drop((observer, folding.machine, folding.lend));
    folding.thread.join().unwrap();

    let burst = heard_at.last().unwrap().duration_since(sent_at[0]);
    let mut taken: Vec<Duration> = sent_at
        .iter()
        .zip(&heard_at)
        .map(|(sent, heard)| heard.saturating_duration_since(*sent))
        .collect();
    taken.sort_unstable();
    println!(
        "instrument_view_backlog records={} readers_asked={asked} samples={} floor_us={} p50_us={} p99_us={} max_us={} max_backlog={max_backlog} burst_ms={:.3} {}",
        records.len(),
        taken.len(),
        taken[0].as_micros(),
        taken[percentile(taken.len(), 50)].as_micros(),
        taken[percentile(taken.len(), 99)].as_micros(),
        taken[taken.len() - 1].as_micros(),
        burst.as_secs_f64() * 1_000.0,
        machine()
    );
}

/// Every line of a city `init` formed with `BURST` signal lines written
/// after its own, parsed back the way the writer's observer hands them.
fn burst_over(city_root: &std::path::Path) -> Vec<EventRecord> {
    let report = init_city(city_root).unwrap();
    let (mut ledger, _report) =
        storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(1_700_000_000_000)).unwrap();
    ledger.append_all((0..BURST).map(signal).collect()).unwrap();
    drop(ledger);
    storage::read_raw_lines_at(&report.ledger_dir)
        .unwrap()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect()
}

/// A signal line as `collab` writes it.
fn signal(n: u64) -> EventDraft {
    let mut body = serde_json::Map::new();
    body.insert("n".to_owned(), serde_json::Value::from(n));
    body.insert(
        "note".to_owned(),
        serde_json::Value::String("a line about the shape of ordinary work".to_owned()),
    );
    let t = TimeMs::new(1_700_000_000_000 + n);
    let room = Address::parse("instrument/room").unwrap();
    let signal = collab::Signal::new(
        kernel::event::record::SignalId::parse(&format!("instrument-s{n}")).unwrap(),
        kernel::event::record::SignalKind::Mention,
        "instrument".to_owned(),
        room.clone(),
        kernel::Version::new(1),
        Payload::new(body).unwrap(),
        t,
    )
    .unwrap();
    EventDraft {
        run: RunId::CITY,
        t,
        who: "instrument".to_owned(),
        addr: Some(room),
        kind: EventKind::SignalEnqueued,
        data: signal.enqueued_payload().unwrap(),
        ig: false,
    }
}

/// The nearest-rank index of `percent` in a sorted vector of `count`.
fn percentile(count: usize, percent: usize) -> usize {
    (count * percent / 100).min(count - 1)
}

fn machine() -> String {
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!(
        "machine={}-{}, {cores} core(s)",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
