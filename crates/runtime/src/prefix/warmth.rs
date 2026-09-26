// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session's keep-warm account: the last real request each prefix
//! carried, and the renewal sent for it when `kernel::keep_warm` says it
//! is due (runtime-SPEC 8-4-2).
//!
//! **Off keeps nothing.** Under [`KeepWarm::Off`] a request is not even
//! cloned, so the default setting costs no memory per session and has no
//! renewal it could send.

use std::collections::BTreeMap;

use kernel::keep_warm::{CacheUse, KeepWarm, renewal_due};
use kernel::{AxError, B3Hash, Ceiling, Model, ModelRequest, ModelReturn};

/// Keep-warm bookkeeping for the prefixes one session has sent.
#[derive(Debug, Clone)]
pub struct Warmth {
    setting: KeepWarm,
    lead_ms: u64,
    kept: BTreeMap<[B3Hash; 4], Kept>,
}

/// A prefix's last real request, and its cache as last seen.
#[derive(Debug, Clone)]
struct Kept {
    request: ModelRequest,
    cache: CacheUse,
}

impl Warmth {
    /// `lead_ms` is the round trip to the provider as the caller measured
    /// it: each renewal leaves that long before the cache expires.
    pub fn new(setting: KeepWarm, lead_ms: u64) -> Warmth {
        Warmth {
            setting,
            lead_ms,
            kept: BTreeMap::new(),
        }
    }

    /// A real request carrying `request.segments` was sent at `at_ms`.
    pub fn sent(&mut self, request: &ModelRequest, at_ms: u64) {
        match self.setting {
            KeepWarm::Off => return,
            KeepWarm::FiveMinute => {}
        }
        self.kept.insert(
            request.segments,
            Kept {
                request: request.clone(),
                cache: CacheUse::sent(at_ms),
            },
        );
    }

    /// The instant the earliest renewal is sent, or `None` when no prefix
    /// is to be renewed and a timer need not wake.
    pub fn next_due(&self) -> Option<u64> {
        self.kept
            .values()
            .filter_map(|kept| renewal_due(self.setting, kept.cache, self.lead_ms))
            .min()
    }

    /// Sends, through `model`, every renewal due by `now_ms` and returns
    /// each one's answer, whose usage the caller records like any other.
    ///
    /// # Errors
    /// The model's failure, unchanged; the prefixes not yet renewed stay
    /// for the next wake.
    pub fn renew_due(
        &mut self,
        model: &mut dyn Model,
        now_ms: u64,
    ) -> Result<Vec<ModelReturn>, AxError> {
        let (setting, lead_ms) = (self.setting, self.lead_ms);
        self.kept
            .retain(|_, kept| renewal_due(setting, kept.cache, lead_ms).is_some());
        let mut answers = Vec::new();
        for kept in self.kept.values_mut() {
            if renewal_due(setting, kept.cache, lead_ms).is_some_and(|due| due <= now_ms) {
                answers.push(model.call(&renewal_of(&kept.request))?);
                kept.cache = kept.cache.renewed(now_ms);
            }
        }
        Ok(answers)
    }
}

/// The last real request, asking for one output token: the cache hits on
/// the prefix bytes whatever the ceiling, so one token is the lowest
/// price a renewal can pay.
fn renewal_of(request: &ModelRequest) -> ModelRequest {
    let mut renewal = request.clone();
    renewal.chat.max_tokens = Ceiling::new(1);
    renewal
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use kernel::consts_external::PROMPT_CACHE_TTL_SECS;
    use kernel::{BuildingPolicy, ChatRequest, ContentBlock};

    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;
    const LEAD_MS: u64 = 2_000;
    const USED_AT: u64 = 1_000_000;

    /// Counts what reaches the provider, and keeps each request it saw.
    #[derive(Default)]
    struct Provider {
        seen: Vec<ModelRequest>,
    }

    impl Model for Provider {
        fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.seen.push(req.clone());
            Ok(ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "warm".to_owned(),
                }])
                .unwrap(),
                Vec::new(),
            ))
        }
    }

    fn request() -> ModelRequest {
        ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"prefix"); 4],
            chat: ChatRequest::empty("script", Ceiling::new(512).unwrap()),
        }
    }

    /// Asks `warmth` for renewals at every instant a timer could wake,
    /// from the real use to well past one cache lifetime.
    fn wake_through_two_lifetimes(warmth: &mut Warmth, provider: &mut Provider) {
        for now in (USED_AT..=USED_AT + 2 * TTL_MS).step_by(1_000) {
            warmth.renew_due(provider, now).unwrap();
        }
    }

    #[test]
    fn the_default_setting_sends_no_request_of_its_own() {
        let mut warmth = Warmth::new(KeepWarm::default(), LEAD_MS);
        let mut provider = Provider::default();
        warmth.sent(&request(), USED_AT);
        assert_eq!(warmth.next_due(), None);
        wake_through_two_lifetimes(&mut warmth, &mut provider);
        assert!(
            provider.seen.is_empty(),
            "{} renewals sent",
            provider.seen.len()
        );
    }

    #[test]
    fn five_minute_resends_the_last_request_once_with_one_output_token() {
        let mut warmth = Warmth::new(KeepWarm::FiveMinute, LEAD_MS);
        let mut provider = Provider::default();
        warmth.sent(&request(), USED_AT);
        assert_eq!(warmth.next_due(), Some(USED_AT + TTL_MS - LEAD_MS));
        wake_through_two_lifetimes(&mut warmth, &mut provider);
        let mut renewal = request();
        renewal.chat.max_tokens = Ceiling::new(1);
        assert_eq!(provider.seen, vec![renewal]);
        assert_eq!(warmth.next_due(), None);
    }
}
