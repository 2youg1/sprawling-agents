// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session's keep-warm account: the last real request each prefix
//! carried, and the renewal sent for it when `kernel::keep_warm` says it
//! is due (`crates/runtime/spec/Prefix.lean` §8-4-2).
//!
//! **Off keeps nothing.** Under [`KeepWarm::Off`] a request is not even
//! cloned, so the default setting costs no memory per session and has no
//! renewal it could send.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::keep_warm::{CacheUse, KeepWarm, renewal_due};
use kernel::{
    AxError, B3Hash, Ceiling, EarlyCalls, Increments, Model, ModelRequest, ModelReturn, TimeMs,
};

/// Keep-warm bookkeeping for the prefixes one session has sent.
#[derive(Debug, Clone)]
pub struct Warmth {
    setting: KeepWarm,
    kept: BTreeMap<[B3Hash; 4], Kept>,
}

/// A prefix's last real request, and its cache as last seen.
#[derive(Debug, Clone)]
struct Kept {
    request: ModelRequest<'static>,
    cache: CacheUse,
}

impl Warmth {
    pub fn new(setting: KeepWarm) -> Warmth {
        Warmth {
            setting,
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
                request: request.clone().into_owned(),
                cache: CacheUse::sent(at_ms),
            },
        );
    }

    /// The instant the earliest renewal is sent with the lead `lead`
    /// reads now, or `None` when no prefix is to be renewed and a timer
    /// need not wake.
    pub fn next_due(&self, lead: &Lead) -> Option<u64> {
        self.kept
            .values()
            .filter_map(|kept| renewal_due(self.setting, kept.cache, lead.ms()))
            .min()
    }

    /// Hands every renewal due by `now_ms` to `send`, one at a time.
    /// Every answer comes back in the round, a refusal beside the
    /// answers that preceded it rather than instead of them: their usage
    /// the caller owes the ledger all the same. Nothing more is sent
    /// after a refusal, and a prefix that owes no renewal is let go
    /// here.
    pub fn renew_due(
        &mut self,
        lead: &Lead,
        now_ms: u64,
        mut send: impl FnMut(&ModelRequest) -> Result<ModelReturn, AxError>,
    ) -> RenewalRound {
        let (setting, lead_ms) = (self.setting, lead.ms());
        self.kept
            .retain(|_, kept| renewal_due(setting, kept.cache, lead_ms).is_some());
        let mut round = RenewalRound::default();
        for kept in self.kept.values_mut() {
            if renewal_due(setting, kept.cache, lead_ms).is_some_and(|due| due <= now_ms) {
                match send(&renewal_of(&kept.request)) {
                    Ok(answer) => {
                        round.answers.push(answer);
                        kept.cache = kept.cache.renewed();
                    }
                    Err(refused) => {
                        round.refused = Some(refused);
                        break;
                    }
                }
            }
        }
        round
    }
}

/// What one wake's renewals came back with: every answer the provider
/// gave, and the first refusal when one came. The answers survive a
/// later refusal, because each one's usage is already spent.
#[derive(Debug, Default)]
pub struct RenewalRound {
    /// Each renewal the provider answered, in the order sent.
    pub answers: Vec<ModelReturn>,
    /// The refusal that ended the round, if one did.
    pub refused: Option<AxError>,
}

/// The last real request, asking for one output token: the cache hits on
/// the prefix bytes whatever the ceiling, so one token is the lowest
/// price a renewal can pay.
fn renewal_of(request: &ModelRequest<'static>) -> ModelRequest<'static> {
    let mut renewal = request.clone();
    renewal.chat.max_tokens = Ceiling::new(1);
    renewal
}

/// How long a request to one endpoint takes from leaving the city to the
/// provider starting it, in milliseconds, as last measured; 0 before the
/// first reading.
///
/// A clone shares the reading, so every door that reaches the endpoint
/// renews by the same lead: the network path and the permit gate belong
/// to the endpoint, and one door sees one renewal per real use, too few
/// to keep a reading of its own (`crates/runtime/spec/Prefix.lean` D100).
#[derive(Debug, Clone, Default)]
pub struct Lead(Arc<AtomicU64>);

impl Lead {
    /// The latest reading.
    pub fn ms(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    fn measured(&self, ms: u64) {
        self.0.store(ms, Ordering::Relaxed);
    }
}

/// The door a run's requests enter the keep-warm account by: it owns the
/// adapter a run calls and is itself the [`Model`] the turn loop is
/// handed, so every request that reached the provider is recorded
/// without the turn loop knowing keep-warm exists.
///
/// It forwards every call method of [`Model`] rather than inheriting a
/// default, because a default is correct and only slower: an inherited
/// `call_speculating` drops the early calls, and no read then starts
/// while the model writes (`crates/runtime/spec/Prefix.lean` D97).
///
/// The clock is the caller's, injected from the one sampling point. The
/// door measures its endpoint's [`Lead`] from a renewal's round trip and
/// from a real call's first increment, never from a whole call, whose
/// generation the provider's lifetime does not wait for (D100).
pub struct Warmed<C> {
    model: Box<dyn Model + Send>,
    warmth: Warmth,
    clock: C,
    lead: Lead,
}

impl<C: FnMut() -> Result<TimeMs, AxError>> Warmed<C> {
    /// Wraps `model`, which reaches the endpoint `lead` is read for;
    /// under [`KeepWarm::Off`] it only forwards.
    pub fn new(model: Box<dyn Model + Send>, setting: KeepWarm, clock: C, lead: Lead) -> Warmed<C> {
        Warmed {
            model,
            warmth: Warmth::new(setting),
            clock,
            lead,
        }
    }

    /// See [`Warmth::next_due`]; the lead is the endpoint's reading now.
    pub fn next_due(&self) -> Option<u64> {
        self.warmth.next_due(&self.lead)
    }

    /// Sends every renewal due by `now_ms` through the adapter this door
    /// owns, and takes each one's round trip as the endpoint's lead: a
    /// renewal asks for one token, so its answer follows the prefill.
    /// A clock or adapter failure ends the round as its refusal.
    pub fn renew_due(&mut self, now_ms: u64) -> RenewalRound {
        let Warmed {
            model,
            warmth,
            clock,
            lead,
        } = self;
        let lead: &Lead = lead;
        warmth.renew_due(lead, now_ms, |renewal| {
            let left = clock()?;
            let answer = model.call(renewal)?;
            let back = clock()?;
            lead.measured(back.value().saturating_sub(left.value()));
            Ok(answer)
        })
    }

    /// Makes one real call through `send`, which reports each increment
    /// to the sink it is handed, and records it when the provider
    /// answered. The time to the first increment becomes the endpoint's
    /// lead; it includes the prefill and the first token, so it errs
    /// early, never late. A call with no increment leaves the lead.
    fn recorded(
        &mut self,
        request: &ModelRequest,
        send: impl FnOnce(&mut dyn Model, Increments<'_>) -> Result<ModelReturn, AxError>,
    ) -> Result<ModelReturn, AxError> {
        let Warmed {
            model,
            warmth,
            clock,
            lead,
        } = self;
        let left = clock()?;
        let mut first = None;
        let answer = send(model.as_mut(), &mut |_| {
            if first.is_none() {
                first = Some(clock());
            }
        })?;
        if let Some(first) = first {
            lead.measured(first?.value().saturating_sub(left.value()));
        }
        warmth.sent(request, left.value());
        Ok(answer)
    }
}

impl<C: FnMut() -> Result<TimeMs, AxError>> Model for Warmed<C> {
    fn provider_account(&self) -> Option<kernel::event::record::ProviderAccountBinding> {
        self.model.provider_account()
    }

    fn account_roster(&self) -> Option<kernel::model::AccountRoster> {
        self.model.account_roster()
    }

    fn select_account(&mut self, account: &kernel::ServerLabel) {
        self.model.select_account(account);
    }

    fn prefix_held(&self) -> Option<kernel::event::record::PrefixHeld> {
        self.model.prefix_held()
    }

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, _| model.call(req))
    }

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, first| {
            model.call_streaming(req, &mut |increment| {
                first(increment);
                onto(increment);
            })
        })
    }

    fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
        early: EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, first| {
            model.call_speculating(
                req,
                &mut |increment| {
                    first(increment);
                    onto(increment);
                },
                early,
            )
        })
    }
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
    use kernel::consts_external::{PROMPT_CACHE_RENEWAL_MARGIN_SECS, PROMPT_CACHE_TTL_SECS};
    use kernel::{BuildingPolicy, ChatRequest, ContentBlock, Increment};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;
    const MARGIN_MS: u64 = PROMPT_CACHE_RENEWAL_MARGIN_SECS * 1000;
    const LEAD_MS: u64 = 2_000;
    const USED_AT: u64 = 1_000_000;

    /// Counts what reaches the provider, and keeps each request it saw.
    #[derive(Default)]
    struct Provider {
        seen: Vec<ModelRequest<'static>>,
    }

    impl Model for Provider {
        fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.seen.push(req.clone().into_owned());
            Ok(ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text {
                    text: "warm".to_owned(),
                }])
                .unwrap(),
                Vec::new(),
            ))
        }
    }

    fn request() -> ModelRequest<'static> {
        ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"prefix"); 4],
            chat: ChatRequest::empty("script", Ceiling::new(512).unwrap()),
        }
    }

    #[test]
    fn a_later_refusal_keeps_the_answers_before_it() {
        let mut warmth = Warmth::new(KeepWarm::FiveMinute);
        let mut first = request();
        first.segments = [B3Hash::digest(b"one"); 4];
        let mut second = request();
        second.segments = [B3Hash::digest(b"two"); 4];
        warmth.sent(&first, USED_AT);
        warmth.sent(&second, USED_AT);
        let due = warmth.next_due(&lead()).unwrap();
        let mut calls = 0;
        let round = warmth.renew_due(&lead(), due, |_| {
            calls += 1;
            if calls == 1 {
                Ok(ModelReturn::bare(
                    kernel::model::message_payload(&[ContentBlock::Text {
                        text: "warm".to_owned(),
                    }])
                    .unwrap(),
                    Vec::new(),
                ))
            } else {
                Err(AxError::failure(
                    kernel::AxCode::Provider,
                    "renew a cached prefix",
                    "rate limited",
                )
                .with_recovery("renewal is skipped until the next real request"))
            }
        });
        assert_eq!(round.answers.len(), 1, "the answered renewal survives");
        assert!(round.refused.is_some(), "the refusal ends the round");
        assert_eq!(calls, 2, "nothing more is sent after the refusal");
        assert!(warmth.next_due(&lead()).is_some(), "the refused prefix stays");
    }

    /// An endpoint's lead that has read [`LEAD_MS`].
    fn lead() -> Lead {
        let lead = Lead::default();
        lead.measured(LEAD_MS);
        lead
    }

    /// Asks `warmth` for renewals at every instant a timer could wake,
    /// from the real use to well past one cache lifetime.
    fn wake_through_two_lifetimes(warmth: &mut Warmth, provider: &mut Provider) {
        for now in (USED_AT..=USED_AT + 2 * TTL_MS).step_by(1_000) {
            let round = warmth.renew_due(&lead(), now, |renewal| provider.call(renewal));
            assert!(round.refused.is_none(), "{:?}", round.refused);
        }
    }

    #[test]
    fn the_default_setting_sends_no_request_of_its_own() {
        let mut warmth = Warmth::new(KeepWarm::default());
        let mut provider = Provider::default();
        warmth.sent(&request(), USED_AT);
        assert_eq!(warmth.next_due(&lead()), None);
        wake_through_two_lifetimes(&mut warmth, &mut provider);
        assert!(
            provider.seen.is_empty(),
            "{} renewals sent",
            provider.seen.len()
        );
    }

    #[test]
    fn five_minute_resends_the_last_request_once_with_one_output_token() {
        let mut warmth = Warmth::new(KeepWarm::FiveMinute);
        let mut provider = Provider::default();
        warmth.sent(&request(), USED_AT);
        assert_eq!(
            warmth.next_due(&lead()),
            Some(USED_AT + TTL_MS - MARGIN_MS - LEAD_MS)
        );
        wake_through_two_lifetimes(&mut warmth, &mut provider);
        let mut renewal = request();
        renewal.chat.max_tokens = Ceiling::new(1);
        assert_eq!(provider.seen, vec![renewal]);
        assert_eq!(warmth.next_due(&lead()), None);
    }

    /// What reached the provider behind a [`Warmed`] door, read after
    /// the door owns the provider.
    type Seen = std::sync::Arc<std::sync::Mutex<Vec<ModelRequest<'static>>>>;

    struct Behind(Seen);

    impl Model for Behind {
        fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
            let mut provider = Provider::default();
            let answer = provider.call(req);
            self.0.lock().unwrap().push(req.clone().into_owned());
            answer
        }
    }

    fn door(setting: KeepWarm) -> (Warmed<impl FnMut() -> Result<TimeMs, AxError>>, Seen) {
        let seen = Seen::default();
        let clock = || Ok(TimeMs::new(USED_AT));
        let warmed = Warmed::new(
            Box::new(Behind(seen.clone())),
            setting,
            clock,
            Lead::default(),
        );
        (warmed, seen)
    }

    fn wake_door_through_two_lifetimes(
        warmed: &mut Warmed<impl FnMut() -> Result<TimeMs, AxError>>,
    ) {
        for now in (USED_AT..=USED_AT + 2 * TTL_MS).step_by(1_000) {
            let round = warmed.renew_due(now);
            assert!(round.refused.is_none(), "{:?}", round.refused);
        }
    }

    #[test]
    fn the_default_door_forwards_a_run_and_sends_nothing_of_its_own() {
        let (mut warmed, seen) = door(KeepWarm::default());
        warmed.call(&request()).unwrap();
        wake_door_through_two_lifetimes(&mut warmed);
        assert_eq!(*seen.lock().unwrap(), vec![request()]);
    }

    #[test]
    fn a_five_minute_door_renews_what_a_run_sent_through_it_once() {
        let (mut warmed, seen) = door(KeepWarm::FiveMinute);
        warmed.call_streaming(&request(), &mut |_| {}).unwrap();
        assert_eq!(warmed.next_due(), Some(USED_AT + TTL_MS - MARGIN_MS));
        wake_door_through_two_lifetimes(&mut warmed);
        let mut renewal = request();
        renewal.chat.max_tokens = Ceiling::new(1);
        assert_eq!(*seen.lock().unwrap(), vec![request(), renewal]);
    }

    /// How long a real call takes from sending to settling.
    const CALL_MS: u64 = 200_000;
    /// How long a streamed real call takes to say its first word.
    const FIRST_WORD_MS: u64 = 3_000;
    /// How long a renewal takes from sending to settling.
    const RENEWAL_MS: u64 = 2_000;

    /// A provider that moves the door's clock itself: a real call says
    /// its first word [`FIRST_WORD_MS`] after it was sent, when it is
    /// streamed, and settles [`CALL_MS`] after; a renewal settles
    /// [`RENEWAL_MS`] after.
    struct Slow {
        now: Arc<AtomicU64>,
        seen: Seen,
    }

    impl Slow {
        fn answer(
            &mut self,
            req: &ModelRequest,
            onto: Increments<'_>,
        ) -> Result<ModelReturn, AxError> {
            self.seen.lock().unwrap().push(req.clone().into_owned());
            if req.chat.max_tokens == Ceiling::new(1) {
                self.now.fetch_add(RENEWAL_MS, Ordering::SeqCst);
            } else {
                self.now.fetch_add(FIRST_WORD_MS, Ordering::SeqCst);
                onto(&Increment::Said("warm".to_owned()));
                self.now
                    .fetch_add(CALL_MS - FIRST_WORD_MS, Ordering::SeqCst);
            }
            Provider::default().call(req)
        }
    }

    impl Model for Slow {
        fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
            self.answer(req, &mut |_| {})
        }

        fn call_streaming(
            &mut self,
            req: &ModelRequest,
            onto: Increments<'_>,
        ) -> Result<ModelReturn, AxError> {
            self.answer(req, onto)
        }
    }

    /// A five-minute door in front of a [`Slow`] provider, on the clock
    /// the provider moves, reading the endpoint's lead through `lead`.
    fn slow_door(
        lead: Lead,
    ) -> (
        Warmed<impl FnMut() -> Result<TimeMs, AxError>>,
        Arc<AtomicU64>,
        Seen,
    ) {
        let (now, seen) = (Arc::new(AtomicU64::new(USED_AT)), Seen::default());
        let provider = Slow {
            now: now.clone(),
            seen: seen.clone(),
        };
        let read = now.clone();
        let clock = move || Ok(TimeMs::new(read.load(Ordering::SeqCst)));
        let warmed = Warmed::new(Box::new(provider), KeepWarm::FiveMinute, clock, lead);
        (warmed, now, seen)
    }

    #[test]
    fn a_long_call_leaves_its_renewal_due_a_lifetime_after_it_was_sent() {
        let (mut warmed, now, seen) = slow_door(Lead::default());
        warmed.call(&request()).unwrap();
        let landed = now.load(Ordering::SeqCst);
        assert_eq!(warmed.next_due(), Some(USED_AT + TTL_MS - MARGIN_MS));
        assert!(warmed.renew_due(landed).refused.is_none());
        assert_eq!(
            *seen.lock().unwrap(),
            vec![request()],
            "renewed at the landing"
        );

        let due = warmed.next_due().unwrap();
        now.store(due, Ordering::SeqCst);
        assert!(warmed.renew_due(due).refused.is_none());
        let sent_again = now.load(Ordering::SeqCst);
        warmed.call(&request()).unwrap();
        assert_eq!(
            warmed.next_due(),
            Some(sent_again + TTL_MS - RENEWAL_MS - MARGIN_MS)
        );
    }

    #[test]
    fn a_streamed_call_reads_the_lead_at_its_first_word() {
        let (mut warmed, _, _) = slow_door(Lead::default());
        let mut words = 0;
        warmed
            .call_streaming(&request(), &mut |_| words += 1)
            .unwrap();
        assert_eq!(
            (words, warmed.next_due()),
            (1, Some(USED_AT + TTL_MS - FIRST_WORD_MS - MARGIN_MS))
        );
    }

    #[test]
    fn doors_to_one_endpoint_share_its_lead() {
        let lead = Lead::default();
        let (mut renewing, now, _) = slow_door(lead.clone());
        let (mut waiting, _, _) = slow_door(lead);
        waiting.call(&request()).unwrap();
        renewing.call(&request()).unwrap();
        let due = renewing.next_due().unwrap();
        now.store(due, Ordering::SeqCst);
        assert!(renewing.renew_due(due).refused.is_none());
        assert_eq!(
            waiting.next_due(),
            Some(USED_AT + TTL_MS - MARGIN_MS - RENEWAL_MS)
        );
    }
}
