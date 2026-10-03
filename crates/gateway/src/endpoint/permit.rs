// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One endpoint's permit gate, and the model face that passes it before
//! a request leaves (`crates/gateway/Spec.lean` §8-6, D20).
//!
//! Every call takes a ticket and waits in arrival order; only the ticket
//! at the head of the queue asks [`Permits`] for a permit, so who gets
//! the next permit is decided by the ticket and not by which thread the
//! operating system wakes. The order is proved in
//! `crates/gateway/spec/Concurrency.lean` (`grants_follow_arrival`); the
//! proptest below checks this queue against it. The clock is the
//! injected `fn() -> Instant`, so the gate behaves the same on Windows,
//! macOS and Linux.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError, ModelRequest, ModelReturn, ProviderFailureKind};

use crate::concurrency::{IN_FLIGHT_DEFAULT, Permits, Take};

use super::config::Endpoint;

/// The longest a call waits in an endpoint's queue before it is refused.
pub(crate) const QUEUE_WAIT_MAX: Duration = Duration::from_secs(600);

/// The status a provider answers when it wants fewer calls at once.
const TOO_MANY_REQUESTS: u32 = 429;

/// One endpoint's permits and the tickets waiting for them, shared by
/// every clone of its transport.
#[derive(Debug)]
pub(crate) struct Gate {
    queue: Mutex<Queue>,
    turn: Condvar,
}

impl Default for Gate {
    fn default() -> Gate {
        Gate {
            queue: Mutex::new(Queue::over(Permits::new(IN_FLIGHT_DEFAULT))),
            turn: Condvar::new(),
        }
    }
}

impl Gate {
    /// Waits in arrival order for a permit and returns the guard that
    /// holds it.
    ///
    /// # Errors
    /// `E_BACKPRESSURE_SHED` once the call has waited [`QUEUE_WAIT_MAX`];
    /// its ticket leaves the queue and the next one moves up.
    fn admit(
        self: &Arc<Gate>,
        monotonic: fn() -> Instant,
        endpoint: &str,
    ) -> Result<Admitted, AxError> {
        let arrived = monotonic();
        let mut queue = self.lock();
        let ticket = queue.arrive();
        loop {
            let now = monotonic();
            let waited = now.saturating_duration_since(arrived);
            let Some(left) = QUEUE_WAIT_MAX
                .checked_sub(waited)
                .filter(|left| !left.is_zero())
            else {
                queue.shed(ticket);
                drop(queue);
                self.turn.notify_all();
                return Err(shed(endpoint, waited));
            };
            match queue.try_take(ticket, now) {
                Turn::Granted => {
                    drop(queue);
                    self.turn.notify_all();
                    return Ok(Admitted {
                        gate: Arc::clone(self),
                        monotonic,
                    });
                }
                Turn::Wait(until) => {
                    let wait =
                        until.map_or(left, |until| until.saturating_duration_since(now).min(left));
                    queue = self
                        .turn
                        .wait_timeout(queue, wait)
                        .map_or_else(|poisoned| poisoned.into_inner().0, |(queue, _)| queue);
                }
            }
        }
    }

    /// The queue, recovered from a holder that panicked: it holds only
    /// counters and tickets, and refusing every later call to the
    /// endpoint would turn one panic into an outage.
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn shed(endpoint: &str, waited: Duration) -> AxError {
    AxError::failure(
        AxCode::BackpressureShed,
        "wait for a permit to call this endpoint",
        format!("{endpoint} after {} s in its queue", waited.as_secs()),
    )
    .with_recovery(
        "this endpoint allows fewer calls at once than the runs that want it: raise its \
         max_in_flight in the endpoint settings, or dispatch fewer runs at once",
    )
}

/// The permits and the tickets in arrival order.
#[derive(Debug, Clone)]
struct Queue {
    permits: Permits,
    next: u64,
    waiting: VecDeque<u64>,
}

/// What one ticket's attempt answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Granted,
    /// Wait to be woken, or until the instant when there is one.
    Wait(Option<Instant>),
}

impl Queue {
    fn over(permits: Permits) -> Queue {
        Queue {
            permits,
            next: 0,
            waiting: VecDeque::new(),
        }
    }

    /// Issues the next ticket and puts it at the back of the queue.
    fn arrive(&mut self) -> u64 {
        let ticket = self.next;
        self.next = self.next.saturating_add(1);
        self.waiting.push_back(ticket);
        ticket
    }

    /// Takes a permit for `ticket` when it is at the head of the queue.
    fn try_take(&mut self, ticket: u64, now: Instant) -> Turn {
        if self.waiting.front() != Some(&ticket) {
            return Turn::Wait(None);
        }
        match self.permits.take(now) {
            Take::Granted => {
                self.waiting.pop_front();
                Turn::Granted
            }
            Take::Full => Turn::Wait(None),
            Take::WaitUntil(until) => Turn::Wait(Some(until)),
        }
    }

    fn shed(&mut self, ticket: u64) {
        self.waiting.retain(|waiting| *waiting != ticket);
    }
}

/// A permit held for one call: settled with the call's outcome, given
/// back when dropped, on every return path.
#[derive(Debug)]
#[must_use = "dropping the guard gives the permit back before the call ran"]
struct Admitted {
    gate: Arc<Gate>,
    monotonic: fn() -> Instant,
}

impl Admitted {
    /// Tells the permits how the call went: a success counts toward
    /// widening, a 429 narrows and holds off until the provider's
    /// `Retry-After` instant.
    fn settle(&self, outcome: &Result<ModelReturn, AxError>) {
        let now = (self.monotonic)();
        let mut queue = self.gate.lock();
        match outcome {
            Ok(_) => queue.permits.succeeded(now),
            Err(err)
                if err.provider_failure()
                    == Some(ProviderFailureKind::Refused {
                        status: TOO_MANY_REQUESTS,
                    }) =>
            {
                queue
                    .permits
                    .rate_limited(now, err.retry_after_ms().map(Duration::from_millis));
            }
            Err(_) => {}
        }
    }
}

impl Drop for Admitted {
    fn drop(&mut self) {
        self.gate.lock().permits.give_back();
        self.gate.turn.notify_all();
    }
}

/// An endpoint reached as a model: every door passes the endpoint's
/// gate first, and holds the permit until the answer is whole.
pub(crate) struct Gated {
    endpoint: Endpoint,
    gate: Arc<Gate>,
    monotonic: fn() -> Instant,
}

impl Gated {
    pub(crate) fn new(endpoint: Endpoint, gate: Arc<Gate>, monotonic: fn() -> Instant) -> Gated {
        Gated {
            endpoint,
            gate,
            monotonic,
        }
    }

    fn through(
        &mut self,
        door: impl FnOnce(&mut Endpoint) -> Result<ModelReturn, AxError>,
    ) -> Result<ModelReturn, AxError> {
        let admitted = self
            .gate
            .admit(self.monotonic, &self.endpoint.config.base_url)?;
        let outcome = door(&mut self.endpoint);
        admitted.settle(&outcome);
        outcome
    }
}

impl kernel::Model for Gated {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.through(|endpoint| endpoint.call(req))
    }

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.through(|endpoint| endpoint.call_streaming(req, onto))
    }

    fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.through(|endpoint| endpoint.call_speculating(req, onto, early))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code: a counted clock and index-picked tickets"
)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Condvar, Mutex, OnceLock};
    use std::time::{Duration, Instant};

    use kernel::{AxCode, AxError, ModelReturn, Payload, ProviderFailureKind};
    use proptest::prelude::*;

    use super::{Gate, Queue, Turn};
    use crate::concurrency::{MaxInFlight, Permits, WIDEN_AFTER};

    /// One step of a trace over the ticket queue (the Lean model's
    /// `QueueEvent`, with the head's grant split into any waiting
    /// ticket's attempt so a queue that lets another ticket through is
    /// caught).
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
        /// `grants_follow_arrival`: over any trace, the tickets that get a
        /// permit are strictly increasing.
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

    /// A clock that moves a minute on every reading.
    fn counted() -> Instant {
        let base = *BASE.get_or_init(Instant::now);
        base + Duration::from_secs(60 * TICKS.fetch_add(1, Ordering::SeqCst))
    }

    /// A clock that never moves.
    fn still() -> Instant {
        *BASE.get_or_init(Instant::now)
    }

    fn gate_of(cap: u32) -> Arc<Gate> {
        Arc::new(Gate {
            queue: Mutex::new(Queue::over(Permits::new(
                MaxInFlight::try_from(cap).unwrap(),
            ))),
            turn: Condvar::new(),
        })
    }

    const URL: &str = "http://127.0.0.1:1/v1/messages";

    /// A call that finds every permit taken waits, and is refused with
    /// `E_BACKPRESSURE_SHED` once its queue time passes the bound; its
    /// ticket leaves, and the permit given back serves the next call.
    #[test]
    fn a_call_that_waits_past_the_bound_is_shed_and_the_next_one_gets_the_permit() {
        let gate = gate_of(1);
        let held = gate.admit(counted, URL).unwrap();
        let done = Arc::new(AtomicBool::new(false));
        // The counted clock moves only when read, so a waker stands in for
        // the minutes a real condvar wait would sleep through.
        let waker = {
            let (gate, done) = (Arc::clone(&gate), Arc::clone(&done));
            std::thread::spawn(move || {
                while !done.load(Ordering::SeqCst) {
                    gate.turn.notify_all();
                    std::thread::sleep(Duration::from_millis(1));
                }
            })
        };
        let refused = gate.admit(counted, URL).unwrap_err();
        done.store(true, Ordering::SeqCst);
        waker.join().unwrap();
        drop(held);
        let next = gate.admit(counted, URL);
        assert_eq!(
            (*refused.code(), gate.lock().waiting.len(), next.is_ok()),
            (AxCode::BackpressureShed, 0, true),
            "{refused}"
        );
    }

    /// A 429 halves the endpoint's limit and holds it until the
    /// provider's Retry-After; a run of successes after that widens it.
    #[test]
    fn a_429_narrows_the_endpoint_and_a_run_of_successes_widens_it_again() {
        let gate = gate_of(4);
        let limited = AxError::provider(ProviderFailureKind::Refused { status: 429 }, "call", URL)
            .retriable_after(0)
            .with_recovery("wait");
        gate.admit(still, URL).unwrap().settle(&Err(limited));
        let narrowed = gate.lock().permits.clone();
        for _ in 0..WIDEN_AFTER {
            gate.admit(still, URL)
                .unwrap()
                .settle(&Ok(ModelReturn::bare(Payload::empty(), vec![])));
        }
        let widened = gate.lock().permits.clone();
        assert_eq!(
            (narrowed.limit(), widened.limit(), widened.in_use()),
            (2, 3, 0)
        );
    }
}
