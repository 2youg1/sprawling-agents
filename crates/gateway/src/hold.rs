// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prefix hold at one endpoint's gate: which call sends the one
//! pre-warm of a cold prefix, which calls wait for it, and when they go
//! on toward a ticket (`crates/gateway/spec/Concurrency.lean` §8-40,
//! D37-D41).
//!
//! A pure decision over the instants it is handed. `endpoint::permit`
//! keeps one [`Holds`] beside its ticket queue under the same lock and
//! wakes the waiting threads on the same condition variable; the
//! properties the Lean model proves are checked against this type by
//! the proptest `holds_keep_the_lean_properties` below.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use kernel::B3Hash;
use kernel::consts_external::PROMPT_CACHE_TTL_SECS;
use kernel::event::record::PrewarmEnd;

/// The longest one pre-warm holds the calls that wait for it, from the
/// moment it is sent (gateway D41).
pub(crate) const HOLD_MAX: Duration = Duration::from_secs(30);

/// How long a key stays warm after a call with it left the gate: the
/// provider's cache lifetime.
const CACHE_LIFETIME: Duration = Duration::from_secs(PROMPT_CACHE_TTL_SECS);

/// What one call's shared prefix is cached under (gateway D38): the
/// provider account and the bytes of the pre-warm that would write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PrefixKey(B3Hash);

impl PrefixKey {
    /// The digest of `account`, a zero byte, and the pre-warm's body.
    /// The zero byte keeps an account name from running into the body.
    pub(crate) fn of(account: &str, body: &[u8]) -> PrefixKey {
        let mut held =
            Vec::with_capacity(account.len().saturating_add(1).saturating_add(body.len()));
        held.extend_from_slice(account.as_bytes());
        held.push(0);
        held.extend_from_slice(body);
        PrefixKey(B3Hash::digest(&held))
    }
}

/// One pre-warm, by number: a waiting call knows the pre-warm it waits
/// for, not only the key, so a later pre-warm of the same key cannot
/// release it by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Prewarm(u64);

/// What a call that reached the gate does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrival {
    /// Take a ticket now.
    Go,
    /// Wait for this pre-warm, at most until `until`.
    Wait { on: Prewarm, until: Instant },
    /// Send this pre-warm, and wait for it, at most until `until`.
    Send { prewarm: Prewarm, until: Instant },
}

/// One endpoint's prefix hold.
#[derive(Debug, Default)]
pub(crate) struct Holds {
    next: u64,
    warm: BTreeMap<PrefixKey, Warm>,
    warming: BTreeMap<PrefixKey, Warming>,
}

/// Until when a key is warm, and the pre-warm whose end last warmed it.
#[derive(Debug, Clone, Copy)]
struct Warm {
    until: Instant,
    by: Option<(Prewarm, PrewarmEnd)>,
}

/// A pre-warm in flight, and when it was sent.
#[derive(Debug, Clone, Copy)]
struct Warming {
    prewarm: Prewarm,
    since: Instant,
}

impl Holds {
    /// A call carrying `key` reached the gate at `now`: a warm key goes
    /// on and stays warm; a key with a pre-warm in flight waits for it;
    /// any other key sends one (gateway D37).
    ///
    /// An instant past what `Instant` can hold sends nothing and holds
    /// nobody, because a hold with no bound is the one thing this type
    /// must not answer.
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

    /// `prewarm` came back at `now`, ended as `ended`: every call waiting
    /// for it goes on, and its key is warm from now. A pre-warm already
    /// taken off by [`Holds::expire`] releases nobody a second time.
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

    /// The clock reads `now`: every pre-warm sent [`HOLD_MAX`] ago or
    /// more releases its calls as timed out (gateway D41), and a warm
    /// mark that ran out is forgotten.
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

    /// Whether `prewarm` still holds the calls that wait for it.
    pub(crate) fn waits(&self, prewarm: Prewarm) -> bool {
        self.warming
            .values()
            .any(|warming| warming.prewarm == prewarm)
    }

    /// How `prewarm` ended, read off its key's warm mark; `None` once a
    /// later pre-warm of the key or the end of the cache lifetime has
    /// replaced it.
    pub(crate) fn ended(&self, key: &PrefixKey, prewarm: Prewarm) -> Option<PrewarmEnd> {
        self.warm
            .get(key)
            .and_then(|warm| warm.by)
            .filter(|(by, _)| *by == prewarm)
            .map(|(_, ended)| ended)
    }

    /// The calls with `key` left the gate at `now`, released by `by`.
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
