// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;

use kernel::AxCode;
use serde_json::{Value, json};

use super::{ScriptedProvider, WireScript};

/// One chat reply in the OpenAI chat format, and one model in the list.
fn script() -> WireScript {
    WireScript::parse(
        &json!({
            "face": "open_ai",
            "models": ["scripted-1"],
            "replies": [{
                "choices": [{
                    "message": { "role": "assistant", "content": "done" },
                    "finish_reason": "stop",
                }],
                "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
            }],
        })
        .to_string(),
    )
    .unwrap()
}

/// The model-list request, carrying a credential the record must not keep.
const LIST: &str =
    "GET /v1/models HTTP/1.1\r\nhost: provider.test\r\nauthorization: Bearer sk-typed\r\n\r\n";

fn chat() -> String {
    let body = r#"{"model":"scripted-1","messages":[{"role":"user","content":"hi"}]}"#;
    format!(
        "POST /v1/chat/completions HTTP/1.1\r\nhost: provider.test\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn provider(record: &Path) -> (ScriptedProvider, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    (
        ScriptedProvider::open(listener, script(), record).unwrap(),
        addr,
    )
}

/// Sends one request, lets the provider answer it, and returns the
/// status line and the parsed body of what came back.
fn exchange(provider: &mut ScriptedProvider, addr: SocketAddr, request: &str) -> (String, Value) {
    let mut client = TcpStream::connect(addr).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    provider.answer_one().unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (
        head.lines().next().unwrap().to_owned(),
        serde_json::from_str(body).unwrap(),
    )
}

#[test]
fn the_same_request_twice_is_recorded_byte_for_byte_alike() {
    let dir = tempfile::tempdir().unwrap();
    let records: Vec<String> = ["first.jsonl", "second.jsonl"]
        .into_iter()
        .map(|name| {
            let record = dir.path().join(name);
            let (mut provider, addr) = provider(&record);
            exchange(&mut provider, addr, LIST);
            exchange(&mut provider, addr, &chat());
            std::fs::read_to_string(&record).unwrap()
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
    let (mut provider, addr) = provider(&dir.path().join("record.jsonl"));
    exchange(&mut provider, addr, &chat());
    let (status, body) = exchange(&mut provider, addr, &chat());
    assert_eq!(
        (status.as_str(), &body["error"]["type"]),
        ("HTTP/1.1 410 Gone", &json!("script_exhausted"))
    );
}

#[test]
fn a_reply_the_city_could_not_read_is_refused_when_the_script_is_read() {
    let refused = WireScript::parse(
        &json!({
            "face": "anthropic",
            "models": [],
            "replies": [{ "choices": [] }],
        })
        .to_string(),
    )
    .err()
    .map(|err| (*err.code(), err.subject().starts_with("replies[0]")));
    assert_eq!(refused, Some((AxCode::ConfigInvalid, true)));
}
