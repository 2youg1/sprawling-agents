// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prefix hold at the gate, end to end through `Gated`
//! (`crates/gateway/spec/Concurrency.lean` §8-40): threads that call one
//! endpoint at once with one cold shared prefix, against a loopback
//! provider reached under a name that is not this machine's, because
//! D39 never holds a call to this machine.

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

/// The name the provider is reached under; the transport resolves it to
/// the loopback port.
const NAME: &str = "provider.test";

const ANSWER: &str = r#"{"content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}"#;

/// How long the provider takes over a pre-warm: long enough that every
/// caller of a wave is at the gate before it answers.
const PREWARM_TAKES: Duration = Duration::from_millis(500);

/// A provider at one name, and every request body it heard.
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

fn is_prewarm(body: &str) -> bool {
    serde_json::from_str::<Value>(body).unwrap()["max_tokens"] == 1
}

/// A turn whose two leading system blocks every resident of a building
/// carries, together well past the default minimum a prefix is cached at.
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

/// `callers` threads, each with a model of its own over the one
/// endpoint, calling at once; each call's outcome and the wait it
/// reports.
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

/// Each call's success, how the pre-warm it waited for ended, and
/// whether its line carries the pre-warm's own answer.
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

/// A cold wave of three: one pre-warm of the shared part goes out
/// first, then the three calls, and each reports the wait; only the
/// call that sent the pre-warm carries its answer.
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

/// A wave whose prefix a call has just used goes out with no hold and
/// no second pre-warm.
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

/// A pre-warm the provider refuses releases all three calls, and each
/// completes on its own request.
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
