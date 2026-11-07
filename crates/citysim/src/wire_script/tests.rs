// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use kernel::AxCode;
use serde_json::{Value, json};

use super::{Answer, Refusal, ScriptedProvider, WireScript};

/// A reply in the OpenAI chat format that calls `status` under `id`.
fn calling(id: &str) -> Value {
    json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "tool_calls": [{
                    "id": id,
                    "type": "function",
                    "function": { "name": "status", "arguments": "{}" },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
}

/// A reply that says something and calls nothing.
fn saying(text: &str) -> Value {
    json!({
        "choices": [{
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
}

/// A whole script: one model in the list, and the runs given.
fn script_of(runs: &[Vec<Value>]) -> Value {
    json!({ "face": "open_ai", "models": ["scripted-1"], "runs": runs })
}

/// One run that calls one tool.
fn script() -> Value {
    script_of(&[vec![calling("call-0-0")]])
}

/// The model-list request, carrying a credential the record must not keep.
const LIST: &str =
    "GET /v1/models HTTP/1.1\r\nhost: provider.test\r\nauthorization: Bearer sk-typed\r\n\r\n";

/// A chat request whose conversation holds `messages`.
fn chat_with(messages: &Value) -> String {
    let body = json!({ "model": "scripted-1", "messages": messages }).to_string();
    format!(
        "POST /v1/chat/completions HTTP/1.1\r\nhost: provider.test\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// The first turn of a run: nothing the script said is in it yet.
fn opening() -> String {
    chat_with(&json!([{ "role": "user", "content": "hi" }]))
}

/// A later turn of the run that was given the call `id`: the city sends
/// the call back, and the result that answers it.
fn after(id: &str) -> String {
    chat_with(&json!([
        { "role": "user", "content": "hi" },
        {
            "role": "assistant",
            "tool_calls": [{
                "id": id,
                "type": "function",
                "function": { "name": "status", "arguments": "{}" },
            }],
        },
        { "role": "tool", "tool_call_id": id, "content": "ok" },
    ]))
}

/// Writes `script` to a file in `dir` and starts a provider playing it.
fn provider(dir: &Path, script: &Value) -> (ScriptedProvider, SocketAddr, PathBuf) {
    let script_path = dir.join("script.json");
    std::fs::write(&script_path, script.to_string()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    (
        ScriptedProvider::open(listener, &script_path, &dir.join("record.jsonl")).unwrap(),
        addr,
        script_path,
    )
}

/// Sends one request, lets the provider answer it, and returns the
/// status line and the parsed body of what came back.
fn exchange(provider: &mut ScriptedProvider, addr: SocketAddr, request: &str) -> (String, Value) {
    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    // A script read again that could not be taken is reported after the
    // turn was answered; nothing else may fail here.
    if let Err(err) = provider.answer_one() {
        assert_eq!(*err.code(), AxCode::ConfigInvalid, "{err}");
    }
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (
        head.lines().next().unwrap().to_owned(),
        serde_json::from_str(body).unwrap(),
    )
}

const OK: &str = "HTTP/1.1 200 OK";

#[test]
fn the_same_request_twice_is_recorded_byte_for_byte_alike() {
    let records: Vec<String> = (0..2)
        .map(|_| {
            let dir = tempfile::tempdir().unwrap();
            let (mut provider, addr, _) = provider(dir.path(), &script());
            exchange(&mut provider, addr, LIST);
            exchange(&mut provider, addr, &opening());
            std::fs::read_to_string(dir.path().join("record.jsonl")).unwrap()
        })
        .collect();
    assert_eq!(
        records[0].lines().count(),
        2,
        "one line per exchange: {}",
        records[0]
    );
    assert_eq!(records[0], records[1]);
    assert!(
        !records[0].contains("sk-typed"),
        "a credential reached the record: {}",
        records[0]
    );
}

#[test]
fn an_exhausted_script_is_refused_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let (mut provider, addr, _) = provider(dir.path(), &script());
    exchange(&mut provider, addr, &opening());
    let (status, body) = exchange(&mut provider, addr, &after("call-0-0"));
    assert_eq!(
        (status.as_str(), &body["error"]["type"]),
        ("HTTP/1.1 410 Gone", &json!("script_exhausted"))
    );
}

/// Two runs whose turns arrive interleaved each get their own replies,
/// and the record says which run every answer came out of.
#[test]
fn two_interleaved_runs_are_each_answered_from_their_own_replies() {
    let dir = tempfile::tempdir().unwrap();
    let runs = [
        vec![calling("call-0-0"), saying("the first run is done")],
        vec![calling("call-1-0"), saying("the second run is done")],
    ];
    let (mut provider, addr, _) = provider(dir.path(), &script_of(&runs));
    let answered: Vec<(String, Value)> =
        [opening(), opening(), after("call-1-0"), after("call-0-0")]
            .iter()
            .map(|request| exchange(&mut provider, addr, request))
            .collect();
    let owed: Vec<(String, Value)> = [&runs[0][0], &runs[1][0], &runs[1][1], &runs[0][1]]
        .into_iter()
        .map(|reply| (OK.to_owned(), reply.clone()))
        .collect();
    assert_eq!(answered, owed);
    let record = std::fs::read_to_string(dir.path().join("record.jsonl")).unwrap();
    let placed: Vec<(Value, Value)> = record
        .lines()
        .map(|line| {
            let exchange: Value = serde_json::from_str(line).unwrap();
            (exchange["run"].clone(), exchange["reply"].clone())
        })
        .collect();
    assert_eq!(
        placed,
        vec![
            (json!(0), json!(0)),
            (json!(1), json!(0)),
            (json!(1), json!(1)),
            (json!(0), json!(1)),
        ]
    );
}

/// A run written into the script after every run in it was opened is
/// opened by the next first turn, and the run already played is not
/// rewritten by it.
#[test]
fn a_run_written_after_the_script_ran_out_is_opened() {
    let dir = tempfile::tempdir().unwrap();
    let first = vec![calling("call-0-0"), saying("the first run is done")];
    let (mut provider, addr, script_path) =
        provider(dir.path(), &script_of(std::slice::from_ref(&first)));
    exchange(&mut provider, addr, &opening());
    let second = vec![saying("written later")];
    std::fs::write(
        &script_path,
        script_of(&[first.clone(), second.clone()]).to_string(),
    )
    .unwrap();
    let answered = [opening(), after("call-0-0")]
        .iter()
        .map(|request| exchange(&mut provider, addr, request))
        .collect::<Vec<(String, Value)>>();
    assert_eq!(
        answered,
        vec![
            (OK.to_owned(), second[0].clone()),
            (OK.to_owned(), first[1].clone()),
        ]
    );
}

/// A script read again that changed a run already being played is not
/// taken: the first turn that asked for it is refused as finding no run.
#[test]
fn a_script_read_again_that_rewrote_a_played_run_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (mut provider, addr, script_path) = provider(dir.path(), &script());
    exchange(&mut provider, addr, &opening());
    std::fs::write(
        &script_path,
        script_of(&[vec![saying("rewritten")], vec![saying("added")]]).to_string(),
    )
    .unwrap();
    let (status, body) = exchange(&mut provider, addr, &opening());
    assert_eq!(
        (status.as_str(), &body["error"]["type"]),
        ("HTTP/1.1 410 Gone", &json!("no_run_left"))
    );
}

#[test]
fn a_reply_the_city_could_not_read_is_refused_when_the_script_is_read() {
    let refused = WireScript::parse(
        &json!({
            "face": "anthropic",
            "models": [],
            "runs": [[{ "choices": [] }]],
        })
        .to_string(),
    )
    .err()
    .map(|err| (*err.code(), err.subject().starts_with("runs[0][0]")));
    assert_eq!(refused, Some((AxCode::ConfigInvalid, true)));
}

/// Each script the stand-in could not play to the end is refused before
/// the city asks anything, naming the reply at fault.
#[test]
fn a_script_whose_runs_cannot_be_told_apart_or_reached_is_refused() {
    let refused: Vec<Option<(AxCode, String)>> = [
        script_of(&[vec![calling("call-0-0")], vec![calling("call-0-0")]]),
        script_of(&[vec![saying("too soon"), calling("call-0-1")]]),
        script_of(&[vec![]]),
    ]
    .iter()
    .map(|script| {
        WireScript::parse(&script.to_string()).err().map(|err| {
            (
                *err.code(),
                err.subject()
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
    })
    .collect();
    assert_eq!(
        refused,
        vec![
            Some((AxCode::ConfigInvalid, "runs[1][0]".to_owned())),
            Some((AxCode::ConfigInvalid, "runs[0][0]".to_owned())),
            Some((AxCode::ConfigInvalid, "runs[0]".to_owned())),
        ]
    );
}

/// The refusal after `refusal` in a walk that reaches every one; a new
/// refusal cannot compile until it is given a place in the walk.
fn after_refusal(refusal: Refusal) -> Option<Refusal> {
    match refusal {
        Refusal::NoModelList => Some(Refusal::ScriptExhausted),
        Refusal::ScriptExhausted => Some(Refusal::NoRunLeft),
        Refusal::NoRunLeft => Some(Refusal::RunsCrossed),
        Refusal::RunsCrossed => Some(Refusal::BodyUnreadable),
        Refusal::BodyUnreadable => Some(Refusal::MethodUnanswered),
        Refusal::MethodUnanswered => None,
    }
}

/// Answers one request on a loopback port with `refusal`, as the
/// stand-in writes it, and returns the URL the request was sent to.
fn refusing_once(refusal: Refusal) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/models", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        let answer = Answer::Refused(refusal);
        let (code, phrase) = answer.status();
        let body = answer.body().to_string();
        let len = body.len();
        write!(
            stream,
            "HTTP/1.1 {code} {phrase}\r\ncontent-type: application/json\r\n\
             content-length: {len}\r\nconnection: close\r\n\r\n{body}"
        )
        .unwrap();
    });
    (url, served)
}

/// citysim D15 and the Exchange part: every refusal the stand-in gives
/// is one the city does not retry, read through the gateway's own
/// `ProviderFailure::retry` on a real exchange rather than a copy of
/// its status set.
#[test]
fn no_refusal_of_the_stand_in_is_retried_by_the_city() {
    let endpoint = gateway::Endpoint::new(
        gateway::EndpointConfig {
            base_url: "http://127.0.0.1:1/v1/chat/completions".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            model: "script".to_owned(),
            auth: gateway::AuthSpec::None,
            extra_headers: Vec::new(),
            overrides: Vec::new(),
            timeout_ms: 5_000,
            stream_idle_timeout_ms: None,
            pricing: None,
            proxying: kernel::Proxying::Never,
        },
        gateway::Redemption::without_images(Box::new(|_reference: &kernel::SecretRef| {
            Err(kernel::AxError::failure(
                AxCode::ConfigInvalid,
                "resolve a credential",
                "none configured",
            )
            .with_recovery("this stand-in authenticates with nothing"))
        })),
    )
    .unwrap();
    let mut next = Some(Refusal::NoModelList);
    while let Some(refusal) = next {
        let (url, served) = refusing_once(refusal);
        let err = endpoint.list_models(&url).unwrap_err();
        served.join().unwrap();
        assert_eq!(
            (refusal.code(), err.retry()),
            (refusal.code(), kernel::Retry::No),
            "{err}"
        );
        next = after_refusal(refusal);
    }
}
