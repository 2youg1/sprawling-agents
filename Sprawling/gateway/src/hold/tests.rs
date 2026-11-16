// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use kernel::event::record::PrewarmEnd;
use proptest::prelude::*;

use super::{Arrival, CACHE_LIFETIME, HOLD_MAX, Holds, PrefixKey, Prewarm};

#[derive(Debug, Clone)]
enum Step {
    Arrive(Option<u8>),

    Settle(usize, PrewarmEnd),

    SettleLate(usize),

    Tick,
}

fn ending() -> impl Strategy<Value = PrewarmEnd> {
    prop_oneof![
        Just(PrewarmEnd::Answered),
        Just(PrewarmEnd::Refused),
        Just(PrewarmEnd::Abandoned),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        4 => proptest::option::weighted(0.9, 0u8..4).prop_map(Step::Arrive),
        2 => (0usize..8, ending()).prop_map(|(pick, ended)| Step::Settle(pick, ended)),
        1 => (0usize..8).prop_map(Step::SettleLate),
        2 => Just(Step::Tick),
    ]
}

fn key(index: u8) -> PrefixKey {
    PrefixKey::of("acct", &[index])
}

#[derive(Debug, Clone, Copy)]
struct Flying {
    key: u8,
    prewarm: Prewarm,
    since: Instant,
}

#[derive(Debug, Clone, Copy)]
struct Waiting {
    call: usize,
    on: Prewarm,
    since: Instant,
}

#[derive(Debug, Default)]
struct Shadow {
    flying: Vec<Flying>,
    waiting: Vec<Waiting>,

    left: BTreeMap<u8, Instant>,

    ended: Vec<(u8, Prewarm, PrewarmEnd)>,

    sent: Vec<(usize, bool)>,
    calls: usize,
}

impl Shadow {
    fn warm(&self, key: u8, now: Instant) -> bool {
        self.left
            .get(&key)
            .is_some_and(|left| now < *left + CACHE_LIFETIME)
    }
}

proptest! {

    #[test]
    fn holds_keep_the_lean_properties(
        trace in proptest::collection::vec((step(), 0u64..=15_000), 0..200),
    ) {
        let mut now = Instant::now();
        let mut holds = Holds::default();
        let mut shadow = Shadow::default();
        for (step, advance_ms) in trace {
            now += Duration::from_millis(advance_ms);
            let mut settled: Option<(Prewarm, PrewarmEnd)> = None;
            match step {
                Step::Arrive(None) => {
                    shadow.sent.push((shadow.calls, true));
                    shadow.calls += 1;
                }
                Step::Arrive(Some(k)) => {
                    let warm = shadow.warm(k, now);
                    let flying = shadow.flying.iter().find(|flying| flying.key == k).copied();
                    let call = shadow.calls;
                    shadow.calls += 1;
                    match holds.arrive(key(k), now) {
                        Arrival::Go => {
                            prop_assert!(warm, "call {call} went on with key {k} cold");
                            shadow.left.insert(k, now);
                            shadow.sent.push((call, true));
                        }
                        Arrival::Wait { on, until } => {
                            prop_assert!(!warm, "call {call} waits on warm key {k}");
                            let flying = flying.ok_or_else(|| TestCaseError::fail(format!(
                                "call {call} waits on key {k} with no pre-warm in flight"
                            )))?;
                            prop_assert_eq!((on, until), (flying.prewarm, flying.since + HOLD_MAX));
                            shadow.waiting.push(Waiting { call, on, since: now });
                        }
                        Arrival::Send { prewarm, until } => {

                            prop_assert!(
                                !warm && flying.is_none(),
                                "call {call} sends a second pre-warm of key {k}: warm {warm}, in flight {flying:?}"
                            );
                            prop_assert_eq!(until, now + HOLD_MAX);
                            shadow.flying.push(Flying { key: k, prewarm, since: now });
                            shadow.waiting.push(Waiting { call, on: prewarm, since: now });
                        }
                    }
                }
                Step::Settle(pick, ended) if !shadow.flying.is_empty() => {
                    let prewarm = shadow.flying[pick % shadow.flying.len()].prewarm;
                    holds.settle(prewarm, ended, now);
                    settled = Some((prewarm, ended));

                    prop_assert!(!holds.waits(prewarm), "{prewarm:?} still holds after settling");
                }
                Step::SettleLate(pick) if !shadow.ended.is_empty() => {
                    let (_, prewarm, _) = shadow.ended[pick % shadow.ended.len()];
                    holds.settle(prewarm, PrewarmEnd::Answered, now);
                }
                Step::Tick => holds.expire(now),
                Step::Settle(..) | Step::SettleLate(_) => {}
            }

            let ticked = matches!(step, Step::Tick);
            for flying in std::mem::take(&mut shadow.flying) {
                if holds.waits(flying.prewarm) {
                    shadow.flying.push(flying);
                    continue;
                }
                let ended = match settled {
                    Some((prewarm, ended)) if prewarm == flying.prewarm => ended,
                    Some(_) | None => {
                        prop_assert!(
                            ticked && flying.since + HOLD_MAX <= now,
                            "{:?} stopped holding on {step:?}", flying.prewarm
                        );
                        PrewarmEnd::TimedOut
                    }
                };
                prop_assert_eq!(holds.ended(&key(flying.key), flying.prewarm), Some(ended));
                shadow.left.insert(flying.key, now);
                shadow.ended.push((flying.key, flying.prewarm, ended));
                for waiting in std::mem::take(&mut shadow.waiting) {
                    if waiting.on == flying.prewarm {
                        shadow.sent.push((waiting.call, false));
                    } else {
                        shadow.waiting.push(waiting);
                    }
                }
            }

            if ticked {
                for waiting in &shadow.waiting {
                    prop_assert!(now < waiting.since + HOLD_MAX, "call {} waited past HOLD_MAX", waiting.call);
                }
            }

            prop_assert_eq!(shadow.waiting.len() + shadow.sent.len(), shadow.calls);
        }

        let direct: Vec<usize> = shadow.sent.iter().filter(|(_, direct)| *direct).map(|(call, _)| *call).collect();
        prop_assert!(direct.windows(2).all(|pair| pair[0] < pair[1]), "unheld calls out of order: {direct:?}");
    }
}
