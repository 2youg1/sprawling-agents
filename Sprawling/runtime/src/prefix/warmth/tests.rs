// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::consts_external::{PROMPT_CACHE_RENEWAL_MARGIN_SECS, PROMPT_CACHE_TTL_SECS};
use kernel::{BuildingPolicy, ChatRequest, ContentBlock, Increment};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const TTL_MS: u64 = PROMPT_CACHE_TTL_SECS * 1000;
const MARGIN_MS: u64 = PROMPT_CACHE_RENEWAL_MARGIN_SECS * 1000;
const LEAD_MS: u64 = 2_000;
const USED_AT: u64 = 1_000_000;

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
    assert!(
        warmth.next_due(&lead()).is_some(),
        "the refused prefix stays"
    );
}

fn lead() -> Lead {
    let lead = Lead::default();
    lead.measured(LEAD_MS);
    lead
}

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

fn wake_door_through_two_lifetimes(warmed: &mut Warmed<impl FnMut() -> Result<TimeMs, AxError>>) {
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

const CALL_MS: u64 = 200_000;

const FIRST_WORD_MS: u64 = 3_000;

const RENEWAL_MS: u64 = 2_000;

struct Slow {
    now: Arc<AtomicU64>,
    seen: Seen,
}

impl Slow {
    fn answer(&mut self, req: &ModelRequest, onto: Increments<'_>) -> Result<ModelReturn, AxError> {
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
