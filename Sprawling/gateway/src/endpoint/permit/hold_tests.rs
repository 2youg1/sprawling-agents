// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use kernel::event::record::{PrefixHeld, PrewarmEnd, PrewarmSent};
use kernel::{
    AxError, ChatMessage, ContentBlock, Model, ModelRequest, ModelReturn, Role, SharedPrefix,
    SystemBlock,
};
use serde_json::{Value, json};

use crate::endpoint::fakes::{answering_provider, config, monotonic, request};
use crate::endpoint::redemption::redemption;
use crate::endpoint::{Endpoint, Transport};

const NAME: &str = "provider.test";

const ANSWER: &str = r#"{"content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}"#;

const PREWARM_TAKES: Duration = Duration::from_millis(500);

struct Standing {
    transport: Transport,
    url: String,
    heard: Arc<Mutex<Vec<String>>>,
}

fn standing(answer: impl Fn(&str) -> (u16, String) + Send + 'static) -> Standing {
    let (addr, heard) = answering_provider(answer);
    Standing {
        transport: Transport::detoured(move |builder| builder.resolve(NAME, addr).no_proxy()),
        url: format!("http://{NAME}:{}/v1/messages", addr.port()),
        heard,
    }
}

#[test]
fn a_call_the_gate_refuses_locally_sends_no_prewarm() {
    let blind = crate::market::MarketSnapshot::builtin()
        .unwrap()
        .lookup("local")
        .unwrap()
        .clone();
    assert_eq!(blind.input, crate::market::InputKinds::Text);
    let endpoint = Endpoint::new(
        crate::endpoint::config::EndpointConfig {
            pricing: Some(blind),
            ..config(&format!("http://{NAME}:1/v1/messages"))
        },
        redemption(),
    )
    .unwrap();
    let mut req = shared_request();
    req.chat.messages.to_mut().push(ChatMessage {
        role: Role::User,
        content: vec![ContentBlock::Image(kernel::ImageRef {
            locator: kernel::Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
            media_type: kernel::ImageType::Png,
            width: 8,
            height: 8,
        })],
    });
    assert!(endpoint.prewarm_for(None, &req).is_none());
}

fn is_prewarm(body: &str) -> bool {
    serde_json::from_str::<Value>(body).unwrap()["max_tokens"] == 1
}

fn shared_request() -> ModelRequest<'static> {
    let long = |word: &str| word.repeat(20_000 / word.len());
    let mut req = request();
    req.chat.system = vec![
        SystemBlock {
            text: long("city "),
            cache: true,
        },
        SystemBlock {
            text: long("building "),
            cache: true,
        },
        SystemBlock {
            text: "resident".to_owned(),
            cache: true,
        },
    ];
    req.chat.shared = SharedPrefix::Leading(NonZeroUsize::new(2).unwrap());
    req.chat.messages = Cow::Owned(vec![ChatMessage {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: "Task: write one section".to_owned(),
        }],
    }]);
    req
}

fn wave(
    standing: &Standing,
    callers: usize,
) -> Vec<(Result<ModelReturn, AxError>, Option<PrefixHeld>)> {
    let start = Arc::new(Barrier::new(callers));
    (0..callers)
        .map(|_| {
            let mut model =
                Endpoint::over(&standing.transport, config(&standing.url), redemption())
                    .unwrap()
                    .gated(&standing.transport, monotonic);
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                let outcome = model.call(&shared_request());
                (outcome, model.prefix_held())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|caller| caller.join().unwrap())
        .collect()
}

fn seen(
    calls: &[(Result<ModelReturn, AxError>, Option<PrefixHeld>)],
) -> Vec<(bool, Option<(PrewarmEnd, bool)>)> {
    let mut seen: Vec<(bool, Option<(PrewarmEnd, bool)>)> = calls
        .iter()
        .map(|(outcome, held)| {
            (
                outcome.is_ok(),
                held.as_ref().map(|held| (held.ended, held.sent.is_some())),
            )
        })
        .collect();
    seen.sort_by_key(|(_, held)| held.is_some_and(|(_, sent)| sent));
    seen
}

#[test]
fn a_cold_wave_sends_one_prewarm_and_then_every_call() {
    let standing = standing(|body| {
        if is_prewarm(body) {
            std::thread::sleep(PREWARM_TAKES);
        }
        (200, ANSWER.to_owned())
    });
    let calls = wave(&standing, 3);
    let heard = standing.heard.lock().unwrap().clone();
    let first: Value = serde_json::from_str(&heard[0]).unwrap();
    assert_eq!(
        (
            heard
                .iter()
                .map(|body| is_prewarm(body))
                .collect::<Vec<_>>(),
            first["system"].as_array().map(Vec::len),
            first["messages"].clone(),
        ),
        (
            vec![true, false, false, false],
            Some(2),
            json!([{ "role": "user", "content": [{ "type": "text", "text": "." }] }]),
        ),
        "what the provider heard, in order"
    );
    assert_eq!(
        seen(&calls),
        vec![
            (true, Some((PrewarmEnd::Answered, false))),
            (true, Some((PrewarmEnd::Answered, false))),
            (true, Some((PrewarmEnd::Answered, true))),
        ]
    );
}

#[test]
fn a_prefix_just_used_is_not_held() {
    let standing = standing(|_| (200, ANSWER.to_owned()));
    let used = wave(&standing, 1);
    let calls = wave(&standing, 3);
    let heard = standing.heard.lock().unwrap().clone();
    assert_eq!(
        (
            heard
                .iter()
                .map(|body| is_prewarm(body))
                .collect::<Vec<_>>(),
            seen(&used),
            seen(&calls),
        ),
        (
            vec![true, false, false, false, false],
            vec![(true, Some((PrewarmEnd::Answered, true)))],
            vec![(true, None), (true, None), (true, None)],
        )
    );
}

#[test]
fn a_refused_prewarm_releases_every_call() {
    let standing = standing(|body| {
        if is_prewarm(body) {
            std::thread::sleep(PREWARM_TAKES);
            return (429, "{}".to_owned());
        }
        (200, ANSWER.to_owned())
    });
    let calls = wave(&standing, 3);
    let refusal = calls
        .iter()
        .find_map(|(_, held)| match held.as_ref()?.sent.as_ref()? {
            PrewarmSent::Refused { refused } => Some(refused.provider_failure()),
            PrewarmSent::Answered { .. } => None,
        });
    assert_eq!(
        (standing.heard.lock().unwrap().len(), seen(&calls), refusal),
        (
            4,
            vec![
                (true, Some((PrewarmEnd::Refused, false))),
                (true, Some((PrewarmEnd::Refused, false))),
                (true, Some((PrewarmEnd::Refused, true))),
            ],
            Some(Some(kernel::ProviderFailureKind::Refused { status: 429 })),
        )
    );
}

#[test]
#[ignore = "a wall-clock instrument: run it by name with --run-ignored only --no-capture"]
#[allow(clippy::print_stdout, reason = "an instrument prints its reading")]
fn instrument_prewarm_for() {
    const ROUNDS: usize = 1_000;
    let endpoint = Endpoint::new(
        config(&format!("http://{NAME}:1/v1/messages")),
        redemption(),
    )
    .unwrap();
    let req = shared_request();
    let mut took: Vec<Duration> = (0..ROUNDS)
        .map(|_| {
            let started = std::time::Instant::now();
            let prewarming = endpoint.prewarm_for(None, &req);
            let took = started.elapsed();
            assert!(prewarming.is_some());
            took
        })
        .collect();
    took.sort_unstable();
    println!(
        "instrument_prewarm_for rounds={ROUNDS} p50_us={} p99_us={} machine={}-{}",
        took[ROUNDS / 2].as_micros(),
        took[ROUNDS * 99 / 100].as_micros(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}
