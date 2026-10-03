// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One endpoint's concurrency permits: how many calls may be in flight
//! to it now, narrowed when the provider answers 429 and widened again
//! once it keeps answering (`crates/gateway/Spec.lean` D17).
//!
//! A pure judgement: it reads only the instant it is handed and the
//! outcome of each call, so it behaves the same on Windows, macOS and
//! Linux. The properties it keeps are proved in
//! `crates/gateway/spec/Concurrency.lean`; the proptest below checks
//! this implementation against them over random traces.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError};

/// The largest `max_in_flight` an endpoint may be configured with.
pub(crate) const IN_FLIGHT_MAX: u32 = 256;

/// The concurrency an endpoint gets when neither its configuration nor
/// its vendor's documentation states one.
pub(crate) const IN_FLIGHT_DEFAULT: MaxInFlight = MaxInFlight(NonZeroU32::MIN.saturating_add(15));

/// How many successes in a row widen a narrowed limit by one.
pub(crate) const WIDEN_AFTER: u32 = 8;

/// A configured concurrency ceiling, 1 to [`IN_FLIGHT_MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaxInFlight(NonZeroU32);

impl TryFrom<u32> for MaxInFlight {
    type Error = AxError;

    fn try_from(value: u32) -> Result<MaxInFlight, AxError> {
        NonZeroU32::new(value)
            .filter(|n| n.get() <= IN_FLIGHT_MAX)
            .map(MaxInFlight)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "set an endpoint's max_in_flight",
                    format!("{value} is outside 1 to {IN_FLIGHT_MAX}"),
                )
                .with_recovery(format!(
                    "give max_in_flight a whole number from 1 to {IN_FLIGHT_MAX}, \
                     or leave it out to take the default"
                ))
            })
    }
}

/// What one attempt to take a permit answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Take {
    /// A permit is taken; give it back once the call has an outcome.
    Granted,
    /// Every permit is in use; wait for one to be given back.
    Full,
    /// The provider asked for nothing to be sent before this instant,
    /// which is always later than the instant the take was asked at.
    WaitUntil(Instant),
}

/// One endpoint's permit state.
#[derive(Debug, Clone)]
pub(crate) struct Permits {
    cap: MaxInFlight,
    limit: u32,
    in_use: u32,
    streak: u32,
    hold_until: Option<Instant>,
}

impl Permits {
    /// Every permit open, up to `cap`.
    pub(crate) fn new(cap: MaxInFlight) -> Permits {
        Permits {
            cap,
            limit: cap.0.get(),
            in_use: 0,
            streak: 0,
            hold_until: None,
        }
    }

    /// Takes a permit when one is free and the provider has not asked
    /// this endpoint to hold off; otherwise says what to wait for.
    pub(crate) fn take(&mut self, now: Instant) -> Take {
        match self.hold_until {
            Some(until) if now < until => Take::WaitUntil(until),
            Some(_) | None if self.in_use < self.limit => {
                self.in_use = self.in_use.saturating_add(1);
                Take::Granted
            }
            Some(_) | None => Take::Full,
        }
    }

    /// Gives one permit back, after the call it covered has an outcome.
    pub(crate) fn give_back(&mut self) {
        self.in_use = self.in_use.saturating_sub(1);
    }

    /// The provider answered 429: halve the limit (at least 1), restart
    /// the run of successes, and hold off until `retry_after` has passed
    /// when the provider said how long.
    pub(crate) fn rate_limited(&mut self, now: Instant, retry_after: Option<Duration>) {
        self.limit = (self.limit / 2).max(1);
        self.streak = 0;
        if let Some(until) = retry_after.and_then(|wait| now.checked_add(wait)) {
            self.hold_until = Some(self.hold_until.map_or(until, |held| held.max(until)));
        }
    }

    /// A call succeeded: after [`WIDEN_AFTER`] in a row, and never before
    /// the instant the provider asked to be left alone until, the limit
    /// grows by one up to the configured ceiling.
    pub(crate) fn succeeded(&mut self, now: Instant) {
        let streak = self.streak.saturating_add(1);
        if streak < WIDEN_AFTER {
            self.streak = streak;
        } else if self.hold_until.is_none_or(|until| until <= now) && self.limit < self.cap.0.get()
        {
            self.limit = self.limit.saturating_add(1);
            self.streak = 0;
        }
    }

    /// How many calls may be in flight now.
    #[cfg(test)]
    pub(crate) fn limit(&self) -> u32 {
        self.limit
    }

    /// How many permits are taken now.
    #[cfg(test)]
    pub(crate) fn in_use(&self) -> u32 {
        self.in_use
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code: a proptest samples one base instant and offsets it"
)]
mod tests {
    use std::time::{Duration, Instant};

    use proptest::prelude::*;

    use super::{MaxInFlight, Permits, Take, WIDEN_AFTER};

    /// One step of a trace, its instant an offset in milliseconds from
    /// the trace's base (the Lean model's `Event`).
    #[derive(Debug, Clone)]
    enum Event {
        Take(u64),
        GiveBack,
        RateLimited(u64, Option<u64>),
        Success(u64),
        Tick,
    }

    fn event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0u64..10_000).prop_map(Event::Take),
            Just(Event::GiveBack),
            ((0u64..10_000), proptest::option::of(0u64..5_000))
                .prop_map(|(at, wait)| Event::RateLimited(at, wait)),
            (0u64..10_000).prop_map(Event::Success),
            Just(Event::Tick),
        ]
    }

    proptest! {
        /// The five properties of `crates/gateway/spec/Concurrency.lean`,
        /// checked after every step of a random trace.
        #[test]
        fn permits_keep_the_lean_properties(
            cap in 1u32..=12,
            trace in proptest::collection::vec(event(), 0..200),
        ) {
            let base = Instant::now();
            let at = |ms: u64| base + Duration::from_millis(ms);
            let cap = MaxInFlight::try_from(cap).unwrap();
            let mut permits = Permits::new(cap);
            for step in trace {
                let before = permits.clone();
                match step {
                    Event::Take(ms) => match permits.take(at(ms)) {
                        Take::Granted => prop_assert!(permits.in_use <= permits.limit),
                        Take::Full => prop_assert!(before.in_use >= before.limit),
                        Take::WaitUntil(until) => prop_assert!(until > at(ms)),
                    },
                    Event::GiveBack => permits.give_back(),
                    Event::RateLimited(ms, wait) => {
                        permits.rate_limited(at(ms), wait.map(Duration::from_millis));
                    }
                    Event::Success(ms) => permits.succeeded(at(ms)),
                    Event::Tick => {}
                }
                prop_assert!(1 <= permits.limit && permits.limit <= cap.0.get());
                prop_assert!(permits.streak < WIDEN_AFTER);
                prop_assert!(before.hold_until <= permits.hold_until);
                if permits.limit > before.limit {
                    let Event::Success(ms) = step else {
                        return Err(TestCaseError::fail("the limit grew on a step that is no success"));
                    };
                    prop_assert!(before.hold_until.is_none_or(|until| until <= at(ms)));
                    prop_assert_eq!(permits.limit, before.limit + 1);
                }
            }
        }
    }

    /// Eight successes after a narrowing widen by one, and not while the
    /// provider's Retry-After still holds.
    #[test]
    fn a_narrowed_limit_widens_after_the_retry_after_instant() {
        let base = Instant::now();
        let mut permits = Permits::new(MaxInFlight::try_from(4).unwrap());
        permits.rate_limited(base, Some(Duration::from_secs(10)));
        for _ in 0..WIDEN_AFTER {
            permits.succeeded(base + Duration::from_secs(1));
        }
        let held = permits.limit();
        permits.succeeded(base + Duration::from_secs(11));
        assert_eq!((held, permits.limit()), (2, 3));
    }

    #[test]
    fn max_in_flight_takes_one_to_two_hundred_fifty_six() {
        assert_eq!(
            [0, 1, 256, 257].map(|n| MaxInFlight::try_from(n).is_ok()),
            [false, true, true, false]
        );
    }
}
