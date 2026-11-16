// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::consts_external::{PROMPT_CACHE_RENEWAL_MARGIN_SECS, PROMPT_CACHE_TTL_SECS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum KeepWarm {

    #[default]
    Off,

    FiveMinute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheUse {
    used_ms: u64,
    renewal: Renewal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Renewal {
    Owed,
    Spent,
}

impl CacheUse {

    pub fn sent(at_ms: u64) -> CacheUse {
        CacheUse {
            used_ms: at_ms,
            renewal: Renewal::Owed,
        }
    }

    #[must_use]
    pub fn renewed(self) -> CacheUse {
        CacheUse {
            renewal: Renewal::Spent,
            ..self
        }
    }
}

pub fn renewal_due(setting: KeepWarm, cache: CacheUse, lead_ms: u64) -> Option<u64> {
    match (setting, cache.renewal) {
        (KeepWarm::Off, Renewal::Owed | Renewal::Spent)
        | (KeepWarm::FiveMinute, Renewal::Spent) => None,
        (KeepWarm::FiveMinute, Renewal::Owed) => Some(
            cache
                .used_ms
                .checked_add(PROMPT_CACHE_TTL_SECS.checked_mul(1000)?)?
                .saturating_sub(PROMPT_CACHE_RENEWAL_MARGIN_SECS.checked_mul(1000)?)
                .saturating_sub(lead_ms),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, reason = "test code")]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;
    const MARGIN_MS: u64 = PROMPT_CACHE_RENEWAL_MARGIN_SECS * 1000;
    const LEAD_MS: u64 = 2_000;

    #[test]
    fn the_default_setting_plans_no_request_of_its_own() {
        let use_at = 1_000_000;
        let cases = [
            CacheUse::sent(use_at),
            CacheUse::sent(use_at).renewed(),
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
        assert_eq!(first, Some(use_at + TTL_MS - MARGIN_MS - LEAD_MS));
        let renewed = CacheUse::sent(use_at).renewed();
        assert_eq!(renewal_due(KeepWarm::FiveMinute, renewed, LEAD_MS), None);
    }

    #[derive(Debug, Clone, Copy)]
    enum Event {
        Used { at_ms: u64 },
        Woke { now_ms: u64, lead_ms: u64 },
    }

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
                        cache = Some((used_ms, kept.renewed()));
                    }
                }
            }
        }
    }
}
