// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use kernel::B3Hash;
use kernel::consts_external::PROMPT_CACHE_TTL_SECS;
use kernel::event::record::PrewarmEnd;

pub(crate) const HOLD_MAX: Duration = Duration::from_secs(30);

const CACHE_LIFETIME: Duration = Duration::from_secs(PROMPT_CACHE_TTL_SECS);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PrefixKey(B3Hash);

impl PrefixKey {
    pub(crate) fn of(account: &str, body: &[u8]) -> PrefixKey {
        let mut held =
            Vec::with_capacity(account.len().saturating_add(1).saturating_add(body.len()));
        held.extend_from_slice(account.as_bytes());
        held.push(0);
        held.extend_from_slice(body);
        PrefixKey(B3Hash::digest(&held))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Prewarm(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrival {
    Go,

    Wait { on: Prewarm, until: Instant },

    Send { prewarm: Prewarm, until: Instant },
}

#[derive(Debug, Default)]
pub(crate) struct Holds {
    next: u64,
    warm: BTreeMap<PrefixKey, Warm>,
    warming: BTreeMap<PrefixKey, Warming>,
}

#[derive(Debug, Clone, Copy)]
struct Warm {
    until: Instant,
    by: Option<(Prewarm, PrewarmEnd)>,
}

#[derive(Debug, Clone, Copy)]
struct Warming {
    prewarm: Prewarm,
    since: Instant,
}

impl Holds {
    pub(crate) fn arrive(&mut self, key: PrefixKey, now: Instant) -> Arrival {
        if let Some(warm) = self.warm.get_mut(&key)
            && now < warm.until
        {
            if let Some(until) = now.checked_add(CACHE_LIFETIME) {
                warm.until = until;
            }
            return Arrival::Go;
        }
        if let Some(warming) = self.warming.get(&key) {
            return warming
                .since
                .checked_add(HOLD_MAX)
                .map_or(Arrival::Go, |until| Arrival::Wait {
                    on: warming.prewarm,
                    until,
                });
        }
        let Some(until) = now.checked_add(HOLD_MAX) else {
            return Arrival::Go;
        };
        let prewarm = Prewarm(self.next);
        self.next = self.next.wrapping_add(1);
        self.warming.insert(
            key,
            Warming {
                prewarm,
                since: now,
            },
        );
        Arrival::Send { prewarm, until }
    }

    pub(crate) fn settle(&mut self, prewarm: Prewarm, ended: PrewarmEnd, now: Instant) {
        let settled = self
            .warming
            .iter()
            .find(|(_, warming)| warming.prewarm == prewarm)
            .map(|(key, _)| *key);
        if let Some(key) = settled {
            self.warming.remove(&key);
            self.warmed(key, (prewarm, ended), now);
        }
    }

    pub(crate) fn expire(&mut self, now: Instant) {
        let lapsed: Vec<(PrefixKey, Prewarm)> = self
            .warming
            .iter()
            .filter(|(_, warming)| {
                warming
                    .since
                    .checked_add(HOLD_MAX)
                    .is_none_or(|until| until <= now)
            })
            .map(|(key, warming)| (*key, warming.prewarm))
            .collect();
        self.warm.retain(|_, warm| now < warm.until);
        for (key, prewarm) in lapsed {
            self.warming.remove(&key);
            self.warmed(key, (prewarm, PrewarmEnd::TimedOut), now);
        }
    }

    pub(crate) fn waits(&self, prewarm: Prewarm) -> bool {
        self.warming
            .values()
            .any(|warming| warming.prewarm == prewarm)
    }

    pub(crate) fn ended(&self, key: &PrefixKey, prewarm: Prewarm) -> Option<PrewarmEnd> {
        self.warm
            .get(key)
            .and_then(|warm| warm.by)
            .filter(|(by, _)| *by == prewarm)
            .map(|(_, ended)| ended)
    }

    fn warmed(&mut self, key: PrefixKey, by: (Prewarm, PrewarmEnd), now: Instant) {
        match now.checked_add(CACHE_LIFETIME) {
            Some(until) => {
                self.warm.insert(
                    key,
                    Warm {
                        until,
                        by: Some(by),
                    },
                );
            }
            None => {
                self.warm.remove(&key);
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code: a shadow of the Lean model over a counted clock"
)]
mod tests;
