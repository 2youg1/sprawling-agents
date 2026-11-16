// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kernel::event::record::{PrefixHeld, PrewarmEnd, PrewarmSent};
use kernel::{AxCode, AxError, ModelRequest, ModelReturn, ProviderFailureKind};

use crate::concurrency::{IN_FLIGHT_DEFAULT, MaxInFlight, Permits, Take};
use crate::hold::{Arrival, Holds, PrefixKey, Prewarm};

use super::config::Endpoint;
use super::prewarm::Prewarming;

pub(crate) const QUEUE_WAIT_MAX: Duration = Duration::from_secs(600);

const TOO_MANY_REQUESTS: u32 = 429;

#[derive(Debug)]
pub(crate) struct Gate {
    state: Mutex<Admission>,
    turn: Condvar,
}

#[derive(Debug)]
struct Admission {
    queue: Queue,
    holds: Holds,
}

impl Default for Gate {
    fn default() -> Gate {
        Gate::admitting(IN_FLIGHT_DEFAULT)
    }
}

impl Gate {
    pub(crate) fn admitting(ceiling: MaxInFlight) -> Gate {
        Gate {
            state: Mutex::new(Admission {
                queue: Queue::over(Permits::new(ceiling)),
                holds: Holds::default(),
            }),
            turn: Condvar::new(),
        }
    }

    fn admit(
        self: &Arc<Gate>,
        monotonic: fn() -> Instant,
        bound: Duration,
        endpoint: &str,
    ) -> Result<Admitted, AxError> {
        let arrived = monotonic();
        let mut state = self.lock();
        let ticket = state.queue.arrive();
        loop {
            let now = monotonic();
            let waited = now.saturating_duration_since(arrived);
            let Some(left) = bound.checked_sub(waited).filter(|left| !left.is_zero()) else {
                state.queue.shed(ticket);
                drop(state);
                self.turn.notify_all();
                return Err(shed(endpoint, waited));
            };
            match state.queue.try_take(ticket, now) {
                Turn::Granted => {
                    drop(state);
                    self.turn.notify_all();
                    return Ok(Admitted {
                        gate: Arc::clone(self),
                        monotonic,
                    });
                }
                Turn::Wait(until) => {
                    let wait =
                        until.map_or(left, |until| until.saturating_duration_since(now).min(left));
                    state = self.sleep(state, wait);
                }
            }
        }
    }

    fn arrive(&self, key: PrefixKey, now: Instant) -> Arrival {
        let mut state = self.lock();
        state.holds.expire(now);
        let arrival = state.holds.arrive(key, now);
        drop(state);
        self.turn.notify_all();
        arrival
    }

    fn wait_for(
        &self,
        key: &PrefixKey,
        on: Prewarm,
        until: Instant,
        monotonic: fn() -> Instant,
    ) -> PrewarmEnd {
        let mut state = self.lock();
        loop {
            let now = monotonic();
            state.holds.expire(now);
            if !state.holds.waits(on) {
                let ended = state.holds.ended(key, on).unwrap_or(PrewarmEnd::TimedOut);
                drop(state);
                self.turn.notify_all();
                return ended;
            }

            state = self.sleep(state, until.saturating_duration_since(now));
        }
    }

    fn settle_prewarm(&self, prewarm: Prewarm, ended: PrewarmEnd, now: Instant) {
        self.lock().holds.settle(prewarm, ended, now);
        self.turn.notify_all();
    }

    fn sleep<'g>(
        &self,
        state: MutexGuard<'g, Admission>,
        wait: Duration,
    ) -> MutexGuard<'g, Admission> {
        self.turn
            .wait_timeout(state, wait)
            .map_or_else(|poisoned| poisoned.into_inner().0, |(state, _)| state)
    }

    #[cfg(test)]
    pub(crate) fn limit(&self) -> u32 {
        self.lock().queue.permits.limit()
    }

    fn lock(&self) -> MutexGuard<'_, Admission> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
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

#[derive(Debug, Clone)]
struct Queue {
    permits: Permits,
    next: u64,
    waiting: VecDeque<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Granted,

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

    fn arrive(&mut self) -> u64 {
        let ticket = self.next;
        self.next = self.next.saturating_add(1);
        self.waiting.push_back(ticket);
        ticket
    }

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

#[derive(Debug)]
#[must_use = "dropping the guard gives the permit back before the call ran"]
struct Admitted {
    gate: Arc<Gate>,
    monotonic: fn() -> Instant,
}

impl Admitted {
    fn settle(&self, outcome: &Result<ModelReturn, AxError>) {
        let now = (self.monotonic)();
        let mut state = self.gate.lock();
        match outcome {
            Ok(_) => state.queue.permits.succeeded(now),
            Err(err)
                if err.provider_failure()
                    == Some(ProviderFailureKind::Refused {
                        status: TOO_MANY_REQUESTS,
                    }) =>
            {
                state
                    .queue
                    .permits
                    .rate_limited(now, err.retry_after_ms().map(Duration::from_millis));
            }
            Err(_) => {}
        }
    }
}

impl Drop for Admitted {
    fn drop(&mut self) {
        self.gate.lock().queue.permits.give_back();
        self.gate.turn.notify_all();
    }
}

#[must_use = "dropping the guard releases the held calls before the pre-warm came back"]
struct Sending {
    gate: Option<Arc<Gate>>,
    prewarm: Prewarm,
    monotonic: fn() -> Instant,
}

impl Sending {
    fn settle(mut self, ended: PrewarmEnd) {
        self.release(ended);
    }

    fn release(&mut self, ended: PrewarmEnd) {
        if let Some(gate) = self.gate.take() {
            gate.settle_prewarm(self.prewarm, ended, (self.monotonic)());
        }
    }
}

impl Drop for Sending {
    fn drop(&mut self) {
        self.release(PrewarmEnd::Abandoned);
    }
}

pub(crate) struct Gated {
    endpoint: Endpoint,
    gate: Arc<Gate>,
    monotonic: fn() -> Instant,

    provider: String,

    accounts: Vec<(kernel::ServerLabel, super::AuthSpec)>,
    active: usize,

    retries: kernel::account_recovery::AccountRetries,

    held: Option<PrefixHeld>,
}

impl Gated {
    pub(crate) fn new(endpoint: Endpoint, gate: Arc<Gate>, monotonic: fn() -> Instant) -> Gated {
        Gated {
            endpoint,
            gate,
            monotonic,
            provider: String::new(),
            accounts: Vec::new(),
            active: 0,
            retries: crate::EndpointTuning::DEFAULTS.account_retries,
            held: None,
        }
    }

    pub(super) fn with_accounts(
        mut self,
        provider: String,
        accounts: Vec<(kernel::ServerLabel, super::AuthSpec)>,
        retries: kernel::account_recovery::AccountRetries,
    ) -> Gated {
        self.provider = provider;
        self.accounts = accounts;
        self.retries = retries;
        self
    }

    fn usable(&self, auth: &super::AuthSpec) -> bool {
        if self.accounts.len() < 2 {
            return true;
        }
        match auth {
            super::AuthSpec::Bearer(reference)
            | super::AuthSpec::Header {
                value: reference, ..
            } => self.endpoint.redemption.holds(reference),
            super::AuthSpec::None => true,
        }
    }

    fn through(
        &mut self,
        req: &ModelRequest,
        door: impl FnOnce(&mut Endpoint) -> Result<ModelReturn, AxError>,
    ) -> Result<ModelReturn, AxError> {
        self.held = None;
        let account = self.accounts.get(self.active).map(|(account, _)| account);
        if let Some(prewarming) = self.endpoint.prewarm_for(account, req) {
            self.held = self.hold(&prewarming);
        }
        let admitted = self.gate.admit(
            self.monotonic,
            QUEUE_WAIT_MAX,
            &self.endpoint.config.base_url,
        )?;
        let outcome = door(&mut self.endpoint);
        admitted.settle(&outcome);
        outcome
    }

    fn hold(&mut self, prewarming: &Prewarming) -> Option<PrefixHeld> {
        let arrived = (self.monotonic)();
        let (ended, sent) = match self.gate.arrive(prewarming.key, arrived) {
            Arrival::Go => return None,
            Arrival::Wait { on, until } => (
                self.gate
                    .wait_for(&prewarming.key, on, until, self.monotonic),
                None,
            ),
            Arrival::Send { prewarm, until } => {
                let (ended, sent) = self.send(prewarming, prewarm, until);
                (ended, Some(sent))
            }
        };
        let waited = (self.monotonic)().saturating_duration_since(arrived);
        Some(PrefixHeld {
            waited_us: u64::try_from(waited.as_micros()).unwrap_or(u64::MAX),
            ended,
            sent,
        })
    }

    fn send(
        &mut self,
        prewarming: &Prewarming,
        prewarm: Prewarm,
        until: Instant,
    ) -> (PrewarmEnd, PrewarmSent) {
        let sending = Sending {
            gate: Some(Arc::clone(&self.gate)),
            prewarm,
            monotonic: self.monotonic,
        };
        let left = until.saturating_duration_since((self.monotonic)());
        let outcome = self
            .gate
            .admit(self.monotonic, left, &self.endpoint.config.base_url)
            .and_then(|admitted| {
                let timeout = until
                    .saturating_duration_since((self.monotonic)())
                    .min(Duration::from_millis(self.endpoint.config.timeout_ms));
                let outcome = self.endpoint.prewarm(prewarming, timeout);
                admitted.settle(&outcome);
                outcome
            });
        let ended = match &outcome {
            Ok(_) => PrewarmEnd::Answered,
            Err(_) if until <= (self.monotonic)() => PrewarmEnd::TimedOut,
            Err(_) => PrewarmEnd::Refused,
        };
        sending.settle(ended);
        let sent = match outcome {
            Ok(answer) => PrewarmSent::Answered {
                usage: answer.usage,
                billed_usd_micros: answer.billed_usd_micros,
            },
            Err(refused) => PrewarmSent::Refused { refused },
        };
        (ended, sent)
    }
}

impl kernel::Model for Gated {
    fn provider_account(&self) -> Option<kernel::event::record::ProviderAccountBinding> {
        self.accounts.get(self.active).map(|(account, _)| {
            kernel::event::record::ProviderAccountBinding {
                provider: self.provider.clone(),
                account: account.clone(),
            }
        })
    }

    fn account_roster(&self) -> Option<kernel::model::AccountRoster> {
        let (current, _) = self.accounts.get(self.active)?;
        Some(kernel::model::AccountRoster {
            provider: self.provider.clone(),
            accounts: self
                .accounts
                .iter()
                .map(|(account, auth)| kernel::model::RosterEntry {
                    account: account.clone(),
                    usable: self.usable(auth),
                })
                .collect(),
            current: current.clone(),
            retries: self.retries,
        })
    }

    fn select_account(&mut self, account: &kernel::ServerLabel) {
        let Some(position) = self
            .accounts
            .iter()
            .position(|(listed, _)| listed == account)
        else {
            return;
        };
        if let Some((_, auth)) = self.accounts.get(position) {
            self.active = position;
            self.endpoint.config.auth = auth.clone();
        }
    }

    fn prefix_held(&self) -> Option<PrefixHeld> {
        self.held.clone()
    }

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.through(req, |endpoint| endpoint.call(req))
    }

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.through(req, |endpoint| endpoint.call_streaming(req, onto))
    }

    fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.through(req, |endpoint| endpoint.call_speculating(req, onto, early))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::disallowed_methods,
    reason = "test code: a loopback provider and threads that call at once"
)]
mod hold_tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code: a counted clock and index-picked tickets"
)]
mod tests;
