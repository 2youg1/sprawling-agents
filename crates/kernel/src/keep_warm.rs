// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Keep-warm: whether, and when, a prompt cache still in use is renewed
//! before the provider lets it expire.
//! The part `crates/kernel/spec/KeepWarm.lean` specifies this module.
//!
//! **Off unless the city enables it.** A renewal is a request the person
//! pays for, so [`KeepWarm::Off`] is the default and plans nothing. Spend
//! is observed where every request's usage is recorded; this decision
//! reads no balance and refuses nothing.

use crate::consts_external::PROMPT_CACHE_TTL_SECS;
use serde::{Deserialize, Serialize};

/// Whether this city renews a warm prompt cache before it expires.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum KeepWarm {
    /// No renewal is ever planned: the city sends no request of its own.
    #[default]
    Off,
    /// A prefix a real request carried within the last cache lifetime is
    /// renewed once, just before that lifetime ends.
    FiveMinute,
}

/// One prefix's cache as last seen, in milliseconds: when a real request
/// carried it, and when the provider last refreshed it. A renewal moves
/// only the second, which is what bounds renewals to one per real use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheUse {
    used_ms: u64,
    refreshed_ms: u64,
}

impl CacheUse {
    /// A real request carrying the prefix was sent at `at_ms`.
    pub fn sent(at_ms: u64) -> CacheUse {
        CacheUse {
            used_ms: at_ms,
            refreshed_ms: at_ms,
        }
    }

    /// A renewal request refreshed the cache at `at_ms`; the last real
    /// use stays where it was.
    #[must_use]
    pub fn renewed(self, at_ms: u64) -> CacheUse {
        CacheUse {
            refreshed_ms: self.refreshed_ms.max(at_ms),
            ..self
        }
    }
}

/// The instant the next renewal request is sent, or `None` when none is.
///
/// `lead_ms` is the round trip to the provider as the caller measured it:
/// the renewal leaves that long before the cache expires, so the provider
/// reads it while the entry is still alive. A renewal whose instant lies
/// more than one cache lifetime after the last real use is not sent.
pub fn renewal_due(setting: KeepWarm, cache: CacheUse, lead_ms: u64) -> Option<u64> {
    match setting {
        KeepWarm::Off => None,
        KeepWarm::FiveMinute => {
            let ttl_ms = PROMPT_CACHE_TTL_SECS.checked_mul(1000)?;
            let due = cache
                .refreshed_ms
                .checked_add(ttl_ms)?
                .saturating_sub(lead_ms);
            (due.saturating_sub(cache.used_ms) <= ttl_ms).then_some(due)
        }
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, reason = "test code")]
mod tests {
    use super::*;
    use crate::consts_external::PROMPT_CACHE_RENEWAL_MARGIN_SECS;
    use proptest::prelude::*;

    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;
    const MARGIN_MS: u64 = PROMPT_CACHE_RENEWAL_MARGIN_SECS * 1000;
    const LEAD_MS: u64 = 2_000;

    #[test]
    fn the_default_setting_plans_no_request_of_its_own() {
        let use_at = 1_000_000;
        let cases = [
            CacheUse::sent(use_at),
            CacheUse::sent(use_at).renewed(use_at + TTL_MS - LEAD_MS),
            CacheUse::sent(0),
        ];
        for cache in cases {
            assert_eq!(renewal_due(KeepWarm::default(), cache, LEAD_MS), None);
        }
    }

    #[test]
    fn five_minute_renews_a_used_prefix_once_just_before_it_expires() {
        let use_at = 1_000_000;
        let first = renewal_due(KeepWarm::FiveMinute, CacheUse::sent(use_at), LEAD_MS);
        assert_eq!(first, Some(use_at + TTL_MS - LEAD_MS));
        let renewed = CacheUse::sent(use_at).renewed(use_at + TTL_MS - LEAD_MS);
        assert_eq!(renewal_due(KeepWarm::FiveMinute, renewed, LEAD_MS), None);
    }

    /// What a door meets, as `Event` in `crates/kernel/spec/KeepWarm.lean`:
    /// a real request carrying the prefix sent at an instant, or a wake at
    /// an instant with the lead its endpoint reads then.
    #[derive(Debug, Clone, Copy)]
    enum Event {
        Used { at_ms: u64 },
        Woke { now_ms: u64, lead_ms: u64 },
    }

    /// Instants within a few lifetimes of each other, and leads from none
    /// to longer than a lifetime, so a wake can fall before, at or after
    /// any renewal's due instant.
    fn event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0..4 * TTL_MS).prop_map(|at_ms| Event::Used { at_ms }),
            (0..4 * TTL_MS, 0..2 * TTL_MS)
                .prop_map(|(now_ms, lead_ms)| Event::Woke { now_ms, lead_ms }),
        ]
    }

    fn setting() -> impl Strategy<Value = KeepWarm> {
        prop_oneof![Just(KeepWarm::Off), Just(KeepWarm::FiveMinute)]
    }

    proptest! {
        /// `at_most_one_renewal_per_use`, `every_renewal_leaves_the_margin`
        /// and `off_sends_nothing` of `crates/kernel/spec/KeepWarm.lean`,
        /// on any trace of uses and wakes with any lead at each wake. The
        /// door is driven here as `Door.step` drives it, and the renewals
        /// are counted from the ones this driver sends.
        #[test]
        fn every_trace_keeps_the_lean_properties(
            setting in setting(),
            events in proptest::collection::vec(event(), 0..40),
        ) {
            let mut cache = None;
            let mut since_use = 0_u32;
            for event in events {
                match event {
                    Event::Used { at_ms } => {
                        cache = Some((at_ms, CacheUse::sent(at_ms)));
                        since_use = 0;
                    }
                    Event::Woke { now_ms, lead_ms } => {
                        let Some((used_ms, kept)) = cache else { continue };
                        let Some(due) = renewal_due(setting, kept, lead_ms) else { continue };
                        if due > now_ms {
                            continue;
                        }
                        prop_assert_eq!(setting, KeepWarm::FiveMinute, "Off sent a renewal");
                        prop_assert!(
                            due <= (used_ms + TTL_MS).saturating_sub(MARGIN_MS),
                            "a renewal due at {due} for a use at {used_ms} leaves no margin"
                        );
                        since_use += 1;
                        prop_assert!(since_use <= 1, "a second renewal for the use at {used_ms}");
                        cache = Some((used_ms, kept.renewed(now_ms)));
                    }
                }
            }
        }
    }
}
