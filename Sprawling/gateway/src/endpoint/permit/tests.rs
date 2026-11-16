// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError, ModelReturn, Payload, ProviderFailureKind};
use proptest::prelude::*;

use super::{Gate, QUEUE_WAIT_MAX, Queue, Turn};
use crate::concurrency::{MaxInFlight, Permits, WIDEN_AFTER};

#[derive(Debug, Clone)]
enum Step {
    Arrive,
    Attempt(usize),
    GiveBack,
    Shed(usize),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        Just(Step::Arrive),
        (0usize..16).prop_map(Step::Attempt),
        Just(Step::GiveBack),
        (0usize..16).prop_map(Step::Shed),
    ]
}

proptest! {

    #[test]
    fn grants_follow_arrival(
        cap in 1u32..=4,
        trace in proptest::collection::vec(step(), 0..200),
    ) {
        let now = Instant::now();
        let mut queue = Queue::over(Permits::new(MaxInFlight::try_from(cap).unwrap()));
        let mut granted: Vec<u64> = Vec::new();
        let mut held = 0u32;
        for step in trace {
            match step {
                Step::Arrive => {
                    queue.arrive();
                }
                Step::Attempt(pick) if !queue.waiting.is_empty() => {
                    let ticket = queue.waiting[pick % queue.waiting.len()];
                    if queue.try_take(ticket, now) == Turn::Granted {
                        granted.push(ticket);
                        held += 1;
                    }
                }
                Step::Shed(pick) if !queue.waiting.is_empty() => {
                    let ticket = queue.waiting[pick % queue.waiting.len()];
                    queue.shed(ticket);
                }
                Step::GiveBack if held > 0 => {
                    queue.permits.give_back();
                    held -= 1;
                }
                Step::Attempt(_) | Step::Shed(_) | Step::GiveBack => {}
            }
        }
        prop_assert!(
            granted.windows(2).all(|pair| pair[0] < pair[1]),
            "permits went to tickets out of arrival order: {granted:?}"
        );
    }
}

static TICKS: AtomicU64 = AtomicU64::new(0);
static BASE: OnceLock<Instant> = OnceLock::new();

fn counted() -> Instant {
    let base = *BASE.get_or_init(Instant::now);
    base + Duration::from_secs(60 * TICKS.fetch_add(1, Ordering::SeqCst))
}

fn still() -> Instant {
    *BASE.get_or_init(Instant::now)
}

fn gate_of(cap: u32) -> Arc<Gate> {
    Arc::new(Gate::admitting(MaxInFlight::try_from(cap).unwrap()))
}

const URL: &str = "http://127.0.0.1:1/v1/messages";

#[test]
fn a_call_that_waits_past_the_bound_is_shed_and_the_next_one_gets_the_permit() {
    let gate = gate_of(1);
    let held = gate.admit(counted, QUEUE_WAIT_MAX, URL).unwrap();
    let done = Arc::new(AtomicBool::new(false));

    let waker = {
        let (gate, done) = (Arc::clone(&gate), Arc::clone(&done));
        std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                gate.turn.notify_all();
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    };
    let refused = gate.admit(counted, QUEUE_WAIT_MAX, URL).unwrap_err();
    done.store(true, Ordering::SeqCst);
    waker.join().unwrap();
    drop(held);
    let next = gate.admit(counted, QUEUE_WAIT_MAX, URL);
    assert_eq!(
        (
            *refused.code(),
            gate.lock().queue.waiting.len(),
            next.is_ok()
        ),
        (AxCode::BackpressureShed, 0, true),
        "{refused}"
    );
}

#[test]
fn a_429_narrows_the_endpoint_and_a_run_of_successes_widens_it_again() {
    let gate = gate_of(4);
    let limited = AxError::provider(ProviderFailureKind::Refused { status: 429 }, "call", URL)
        .retriable_after(0)
        .with_recovery("wait");
    gate.admit(still, QUEUE_WAIT_MAX, URL)
        .unwrap()
        .settle(&Err(limited));
    let narrowed = gate.lock().queue.permits.clone();
    for _ in 0..WIDEN_AFTER {
        gate.admit(still, QUEUE_WAIT_MAX, URL)
            .unwrap()
            .settle(&Ok(ModelReturn::bare(Payload::empty(), vec![])));
    }
    let widened = gate.lock().queue.permits.clone();
    assert_eq!(
        (narrowed.limit(), widened.limit(), widened.in_use()),
        (2, 3, 0)
    );
}

#[test]
#[ignore = "a wall-clock instrument; just bench runs it"]
fn instrument_provider_queue() {
    const HOLD: Duration = Duration::from_millis(2);
    for callers in [16usize, 64] {
        let gate = gate_of(4);
        let mut waits: Vec<Duration> = (0..callers)
            .map(|_| {
                let gate = Arc::clone(&gate);
                std::thread::spawn(move || {
                    let asked = Instant::now();
                    let held = gate
                        .admit(crate::endpoint::fakes::monotonic, QUEUE_WAIT_MAX, URL)
                        .unwrap();
                    let waited = asked.elapsed();
                    std::thread::sleep(HOLD);
                    drop(held);
                    waited
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|lane| lane.join().unwrap())
            .collect();
        waits.sort_unstable();
        let at = |per_mille: usize| waits[(waits.len() * per_mille / 1_000).min(waits.len() - 1)];
        println!(
            "instrument_provider_queue callers={callers} max_in_flight=4 hold_us={} samples={} p50_us={} p99_us={} p999_us={} max_us={} machine={}-{}",
            HOLD.as_micros(),
            waits.len(),
            at(500).as_micros(),
            at(990).as_micros(),
            at(999).as_micros(),
            waits[waits.len() - 1].as_micros(),
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }
}
