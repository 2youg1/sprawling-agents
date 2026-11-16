// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError};

pub(crate) const IN_FLIGHT_MAX: u32 = 256;

pub(crate) const IN_FLIGHT_DEFAULT: MaxInFlight = MaxInFlight(NonZeroU32::MIN.saturating_add(15));

pub(crate) const IN_FLIGHT_LOCAL: MaxInFlight = MaxInFlight(NonZeroU32::MIN.saturating_add(3));

pub(crate) fn vendor_in_flight(base_url: &str) -> MaxInFlight {
    if crate::reach::is_local(base_url) {
        IN_FLIGHT_LOCAL
    } else {
        IN_FLIGHT_DEFAULT
    }
}

pub(crate) const WIDEN_AFTER: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct MaxInFlight(NonZeroU32);

impl From<MaxInFlight> for u32 {
    fn from(ceiling: MaxInFlight) -> u32 {
        ceiling.0.get()
    }
}

impl MaxInFlight {

    #[must_use]
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Take {

    Granted,

    Full,

    WaitUntil(Instant),
}

#[derive(Debug, Clone)]
pub(crate) struct Permits {
    cap: MaxInFlight,
    limit: u32,
    in_use: u32,
    streak: u32,
    hold_until: Option<Instant>,
}

impl Permits {

    pub(crate) fn new(cap: MaxInFlight) -> Permits {
        Permits {
            cap,
            limit: cap.0.get(),
            in_use: 0,
            streak: 0,
            hold_until: None,
        }
    }

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

    pub(crate) fn give_back(&mut self) {
        self.in_use = self.in_use.saturating_sub(1);
    }

    pub(crate) fn rate_limited(&mut self, now: Instant, retry_after: Option<Duration>) {
        self.limit = (self.limit / 2).max(1);
        self.streak = 0;
        if let Some(until) = retry_after.and_then(|wait| now.checked_add(wait)) {
            self.hold_until = Some(self.hold_until.map_or(until, |held| held.max(until)));
        }
    }

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

    #[cfg(test)]
    pub(crate) fn limit(&self) -> u32 {
        self.limit
    }

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

    proptest! {

        #[test]
        fn a_trace_without_narrowing_keeps_in_use_within_the_limit(
            cap in 1u32..=12,
            prefix in proptest::collection::vec(event(), 0..100),
            suffix in proptest::collection::vec(event(), 0..200),
        ) {
            let base = Instant::now();
            let at = |ms: u64| base + Duration::from_millis(ms);
            let mut permits = Permits::new(MaxInFlight::try_from(cap).unwrap());
            let apply = |permits: &mut Permits, step: &Event| match step {
                Event::Take(ms) => {
                    permits.take(at(*ms));
                }
                Event::GiveBack => permits.give_back(),
                Event::RateLimited(ms, wait) => {
                    permits.rate_limited(at(*ms), wait.map(Duration::from_millis));
                }
                Event::Success(ms) => permits.succeeded(at(*ms)),
                Event::Tick => {}
            };
            for step in &prefix {
                apply(&mut permits, step);
            }
            prop_assume!(permits.in_use() <= permits.limit());
            for step in suffix
                .iter()
                .filter(|step| !matches!(step, Event::RateLimited(..)))
            {
                apply(&mut permits, step);
                prop_assert!(permits.in_use() <= permits.limit());
            }
        }
    }

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
