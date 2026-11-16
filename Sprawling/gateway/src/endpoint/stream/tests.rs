// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![cfg(test)]

use super::super::config::EndpointConfig;
use super::super::fakes::{config, fake_stream_provider, paced_stream_provider, request};
use super::super::redemption::redemption;
use super::*;
use std::time::Duration;

fn whole_answer() -> Vec<String> {
    vec![
        serde_json::json!({
            "type": "message_start",
            "message": {"usage": {"input_tokens": 1_000_000}}
        })
        .to_string(),
        serde_json::json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        })
        .to_string(),
        serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "still "}
        })
        .to_string(),
        serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "writing"}
        })
        .to_string(),
        serde_json::json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 0}
        })
        .to_string(),
    ]
}

#[test]
fn a_model_that_keeps_writing_is_never_cut_off_for_writing_at_length() {
    let (url, server) =
        paced_stream_provider(whole_answer(), Duration::from_millis(300), Duration::ZERO);
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            stream_idle_timeout_ms: Some(800),
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let said = std::cell::RefCell::new(String::new());
    let mut onto = |held: &kernel::Increment| {
        if let kernel::Increment::Said(text) = held {
            said.borrow_mut().push_str(text);
        }
    };
    let ret = endpoint.stream(&request(), &mut onto, &mut |_| {}).unwrap();
    assert_eq!(said.borrow().as_str(), "still writing");
    assert_eq!(ret.stop, Some(kernel::StopReason::EndTurn));
    server.join().unwrap();
}

#[test]
fn a_stream_that_falls_silent_is_given_up_on_and_says_for_how_long() {
    let (url, server) = paced_stream_provider(
        whole_answer().into_iter().take(3).collect::<Vec<String>>(),
        Duration::from_millis(20),
        Duration::from_millis(1_500),
    );
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            stream_idle_timeout_ms: Some(400),
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let mut onto = |_held: &kernel::Increment| {};
    let err = endpoint
        .stream(&request(), &mut onto, &mut |_| {})
        .unwrap_err();
    assert_eq!(*err.code(), kernel::AxCode::Provider);
    assert!(
        err.subject().contains("no byte arrived for 400 ms"),
        "a stalled stream names the bound it passed: {}",
        err.subject()
    );
    server.join().unwrap();
}

#[test]
fn increments_reach_the_caller_while_the_body_is_still_arriving() {
    let opening = vec![
        serde_json::json!({
            "type": "message_start",
            "message": {"usage": {"input_tokens": 1_000_000}}
        })
        .to_string(),
        serde_json::json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        })
        .to_string(),
        serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "on "}
        })
        .to_string(),
    ];
    let closing = vec![
        serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "it"}
        })
        .to_string(),
        serde_json::json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 0}
        })
        .to_string(),
    ];
    let (saw_opening, waiting) = std::sync::mpsc::channel();
    let (url, server) = fake_stream_provider(opening, waiting, closing);
    let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();

    let said = std::cell::RefCell::new(Vec::new());
    let mut onto = |held: &kernel::Increment| {
        let kernel::Increment::Said(text) = held else {
            return;
        };
        said.borrow_mut().push(text.clone());

        let _ = saw_opening.send(());
    };
    let ret = endpoint.stream(&request(), &mut onto, &mut |_| {}).unwrap();

    assert!(
        server.join().unwrap(),
        "the provider was still writing when the first increment should have been \
             handed on; `.text()` reads to the end of the body before anything is forwarded"
    );
    assert_eq!(said.borrow().as_slice(), ["on ", "it"]);
    assert_eq!(ret.stop, Some(kernel::StopReason::EndTurn));
}

fn responses_answer(text: &str) -> Value {
    serde_json::json!({
        "status": "completed", "output": [
            {"type": "reasoning", "summary": [{"text": "reason"}]},
            {"type": "message", "content": [{"type": "output_text", "text": text}]},
            {"type": "function_call", "call_id": "call-1", "name": "read", "arguments": "{\"path\":\"notes.md\"}"}
        ],
        "usage": {"input_tokens": 10, "output_tokens": 5,
            "input_tokens_details": {"cached_tokens": 3, "cache_write_tokens": 2}}
    })
}

#[test]
fn responses_terminal_revises_deltas_and_returns_the_complete_reply() {
    use super::super::fakes::fake_provider;
    let answer = responses_answer("revision");
    let (forwarded, waiting) = std::sync::mpsc::channel();
    let (url, server) = fake_stream_provider(
        vec![
            serde_json::json!({"type": "response.output_text.delta", "delta": "draft"}).to_string(),
        ],
        waiting,
        vec![
            serde_json::json!({"type": "response.completed", "response": responses_answer("old")})
                .to_string(),
            serde_json::json!({"type": "response.completed", "response": answer}).to_string(),
            serde_json::json!({"type": "response.completed"}).to_string(),
        ],
    );
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            dialect: kernel::DialectKind::OpenAiResponses,
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let mut increments = Vec::new();
    let streamed = endpoint
        .stream(
            &request(),
            &mut |increment| {
                increments.push(increment.clone());
                forwarded.send(()).unwrap();
            },
            &mut |_| {},
        )
        .unwrap();
    assert!(server.join().unwrap(), "the delta is forwarded before EOF");
    assert_eq!(
        increments,
        vec![kernel::Increment::Said("draft".to_owned())]
    );
    let (url, server) = fake_provider(vec![(200, answer.to_string())], false);
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            dialect: kernel::DialectKind::OpenAiResponses,
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    assert_eq!(streamed, endpoint.call(&request()).unwrap());
    server.join().unwrap();
}

#[test]
fn responses_first_error_keeps_forwarding_deltas_until_eof() {
    let (forwarded, waiting) = std::sync::mpsc::channel();
    let (url, server) = fake_stream_provider(
            vec![
                serde_json::json!({"type": "error", "code": "server_error"}).to_string(),
                serde_json::json!({"type": "response.output_text.delta", "delta": "after error"}).to_string(),
            ], waiting,
            vec![
                serde_json::json!({"type": "error", "code": "invalid_prompt"}).to_string(),
                serde_json::json!({"type": "response.reasoning_text.delta", "delta": "later"}).to_string(),
                serde_json::json!({"type": "response.completed", "response": responses_answer("final")}).to_string(),
            ],
        );
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            dialect: kernel::DialectKind::OpenAiResponses,
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let mut increments = Vec::new();
    let error = endpoint
        .stream(
            &request(),
            &mut |increment| {
                increments.push(increment.clone());
                if increments.len() == 1 {
                    forwarded.send(()).unwrap();
                }
            },
            &mut |_| {},
        )
        .unwrap_err();
    assert!(
        server.join().unwrap(),
        "error must not return before the following delta"
    );
    assert_eq!(
        increments,
        vec![
            kernel::Increment::Said("after error".to_owned()),
            kernel::Increment::Thought("later".to_owned())
        ]
    );
    assert_eq!(error.subject(), "the stream reported server_error");
}

#[test]
fn responses_silence_precedes_a_terminal_and_a_reported_error() {
    let (url, server) = paced_stream_provider(vec![
            serde_json::json!({"type": "response.completed", "response": responses_answer("final")}).to_string(),
            serde_json::json!({"type": "error", "code": "server_error"}).to_string(),
        ], Duration::ZERO, Duration::from_millis(500));
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            dialect: kernel::DialectKind::OpenAiResponses,
            stream_idle_timeout_ms: Some(100),
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let error = endpoint
        .stream(&request(), &mut |_| {}, &mut |_| {})
        .unwrap_err();
    assert_eq!(error.subject(), "no byte arrived for 100 ms");
    server.join().unwrap();
}

#[test]
fn responses_read_failure_precedes_a_terminal_and_a_reported_error() {
    use std::io::{BufRead as _, Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut reader = std::io::BufReader::new(&mut socket);
        let mut line = String::new();
        let mut length = 0;
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<u64>().unwrap();
            }
        }
        std::io::copy(&mut reader.take(length), &mut std::io::sink()).unwrap();
        let body = format!(
            "data: {}

data: {}

",
            serde_json::json!({"type": "response.completed", "response": responses_answer("final")}),
            serde_json::json!({"type": "error", "code": "server_error"})
        );
        write!(socket, "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len() + 100).unwrap();
    });
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            dialect: kernel::DialectKind::OpenAiResponses,
            ..config(&url)
        },
        redemption(),
    )
    .unwrap();
    let error = endpoint
        .stream(&request(), &mut |_| {}, &mut |_| {})
        .unwrap_err();
    assert!(matches!(
        error.provider_failure(),
        Some(kernel::ProviderFailureKind::Cut)
    ));
    server.join().unwrap();
}

fn process_responses_fixture(deltas: usize, delta: &str, terminal: &str) -> usize {
    use kernel::DialectKind;
    let mut peak_retained = 0;
    let mut frames = dialect::StreamFrames::new(DialectKind::OpenAiResponses);
    for line in std::iter::repeat_n(delta, deltas).chain(std::iter::once(terminal)) {
        let frame = frame_of(std::hint::black_box(line)).unwrap();
        std::hint::black_box(dialect::increment_of(DialectKind::OpenAiResponses, &frame));
        assert!(frames.retain_and_complete(frame).unwrap().is_none());
        peak_retained = peak_retained.max(frames.retained_frames());
    }
    let reply =
        dialect::response_from_wire(DialectKind::OpenAiResponses, &frames.finish().unwrap())
            .unwrap();
    std::hint::black_box(reply);
    peak_retained
}

fn responses_fixture(deltas: usize) -> (String, String) {
    use serde_json::json;
    let delta = format!(
        "data: {}",
        json!({"type": "response.output_text.delta", "delta": "x".repeat(64)})
    );
    let terminal = format!(
        "data: {}",
        json!({
            "type": "response.completed", "response": {
                "status": "completed", "output": [{"type": "message", "content": [
                    {"type": "output_text", "text": "final"}]}],
                "usage": {"input_tokens": 10, "output_tokens": deltas}
            }
        })
    );
    (delta, terminal)
}

#[test]
#[ignore = "allocation instrument; run under heaptrack"]
fn instrument_responses_heap_small() {
    let (delta, terminal) = responses_fixture(8);
    std::hint::black_box(process_responses_fixture(8, &delta, &terminal));
}

#[test]
#[ignore = "allocation instrument; run under heaptrack"]
fn instrument_responses_heap_long() {
    let (delta, terminal) = responses_fixture(16_384);
    std::hint::black_box(process_responses_fixture(16_384, &delta, &terminal));
}

#[test]
#[ignore = "wall-clock instrument; just bench runs it"]
#[allow(clippy::disallowed_methods, reason = "instrument samples its clock")]
fn instrument_responses_stream() {
    use std::time::Instant;
    for deltas in [8, 16_384] {
        let (delta, terminal) = responses_fixture(deltas);
        let fixture = kernel::B3Hash::digest(format!("{deltas}:{delta}:{terminal}").as_bytes());
        let mut peak_retained = 0;
        let mut samples = Vec::new();
        for sample in 0..220 {
            let started = Instant::now();
            peak_retained = peak_retained.max(process_responses_fixture(deltas, &delta, &terminal));
            let nanos = started.elapsed().as_nanos();
            if sample >= 20 {
                samples.push(nanos);
                println!(
                    "responses_sample deltas={deltas} sample={} nanos={nanos}",
                    sample - 20
                );
            }
        }
        samples.sort_unstable();
        println!(
            "responses_summary fixture={fixture} deltas={deltas} delta_bytes=64 samples={} floor_ns={} p50_ns={} p99_ns={} max_ns={} peak_retained_frames={peak_retained} memory_source=owned_frame_count network=excluded warmup=20",
            samples.len(),
            samples[0],
            samples[99],
            samples[197],
            samples[199]
        );
    }
}
