// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The loopback providers the endpoint tests stand up, and the one
//! configuration and request they all start from.
//!
//! **Separated from `config` for the reason card-4.1 separated the
//! stream readers from the request writers: they change for different
//! reasons.** What an endpoint is configured with moves when a provider
//! grows a field; what a test has to fake moves when a new failure is
//! worth reproducing - a truncated body, a silent socket, a stream
//! written in two parts. Keeping them in one file had that file answer
//! both questions and cross the size budget while doing it.
//!
//! Every item here is `#[cfg(test)]`: nothing in this module reaches a
//! shipped binary.

#![cfg(test)]
#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helpers"
)]

use std::time::Duration;

use kernel::{B3Hash, BuildingPolicy, ChatRequest, DialectKind, ModelRequest, SecretRef};
use serde_json::Value;

use super::config::{AuthSpec, EndpointConfig};

#[cfg(test)]
/// A one-shot loopback provider: answers `responses` in order, then
/// closes. `cut_mid_body` truncates the JSON body mid-way.
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn fake_provider(
    responses: Vec<(u16, String)>,
    cut_mid_body: bool,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let mut request = String::new();
            loop {
                let n = stream.read(&mut buf).unwrap();
                request.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(head_end) = request.find("\r\n\r\n") {
                    let head = &request[..head_end];
                    let content_length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= head_end + 4 + content_length {
                        break;
                    }
                }
            }
            seen.push(request.clone());
            let payload = if cut_mid_body {
                let cut = body.len() / 2;
                let truncated = &body[..cut];
                format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{truncated}",
                    body.len()
                )
            } else {
                format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            stream.write_all(payload.as_bytes()).unwrap();
            drop(stream);
        }
        seen
    });
    (format!("http://{addr}/v1/messages"), handle)
}

/// How long the streaming fixture waits to hear that its opening frames
/// landed. Long enough that a loaded machine is not called a defect,
/// short enough that a buffering reader fails rather than hangs.
#[cfg(test)]
const SAW_OPENING_WAIT: Duration = Duration::from_secs(2);

/// A provider that answers `text/event-stream` in two parts, so a test
/// can tell a stream forwarded as it arrives from one read to the end
/// and replayed.
///
/// **The server itself is the assertion.** It writes the opening frames,
/// flushes, and then waits to hear that the caller saw them before it
/// writes the rest. A reader that buffers the whole body cannot report
/// anything until the body is complete, so the wait times out and the
/// handle answers `false`. Nothing on the test's side reads a clock, and
/// nothing is inferred from how long anything took.
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn fake_stream_provider(
    opening: Vec<String>,
    saw_opening: std::sync::mpsc::Receiver<()>,
    closing: Vec<String>,
) -> (String, std::thread::JoinHandle<bool>) {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let mut request = String::new();
        loop {
            let n = stream.read(&mut buf).unwrap();
            request.push_str(&String::from_utf8_lossy(&buf[..n]));
            if let Some(head_end) = request.find("\r\n\r\n") {
                let head = &request[..head_end];
                let content_length = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + content_length {
                    break;
                }
            }
        }
        // No content-length: an event stream ends when the connection
        // does, which is what lets the body be written in two parts.
        stream
            .write_all(
                b"HTTP/1.1 200 X\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n",
            )
            .unwrap();
        for frame in &opening {
            stream
                .write_all(format!("data: {frame}\n\n").as_bytes())
                .unwrap();
        }
        stream.flush().unwrap();
        let forwarded = saw_opening.recv_timeout(SAW_OPENING_WAIT).is_ok();
        for frame in &closing {
            stream
                .write_all(format!("data: {frame}\n\n").as_bytes())
                .unwrap();
        }
        stream.write_all(b"data: [DONE]\n\n").unwrap();
        stream.flush().unwrap();
        drop(stream);
        forwarded
    });
    (format!("http://{addr}/v1/messages"), handle)
}

/// A provider that accepts the connection and then says nothing.
///
/// This is the shape of a hang, and it is worse than a refusal: a
/// refusal is an answer the run can act on, while silence stops the
/// worker thread that would have acted. The deadline is the only
/// thing standing between the two, so it gets its own test.
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn silent_provider() -> String {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let held = listener.accept();
        // Hold the connection open without answering, long enough
        // for any deadline under test to pass.
        std::thread::sleep(Duration::from_secs(5));
        drop(held);
    });
    format!("http://{addr}/v1/messages")
}
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn config(url: &str) -> EndpointConfig {
    EndpointConfig {
        base_url: url.to_owned(),
        dialect: DialectKind::Anthropic,
        model: "provider-model".to_owned(),
        auth: AuthSpec::Header {
            name: "x-api-key".to_owned(),
            value: SecretRef::parse("secret:anthropic/api").unwrap(),
        },
        extra_headers: vec![("anthropic-version".to_owned(), "2023-06-01".to_owned())],
        overrides: vec![(
            "/metadata/user_id".to_owned(),
            Value::String("city".to_owned()),
        )],
        timeout_ms: 5_000,
        pricing: Some(
            crate::market::MarketSnapshot::builtin()
                .lookup("claude-sonnet")
                .unwrap()
                .clone(),
        ),
    }
}
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn request() -> ModelRequest {
    ModelRequest {
        policy: BuildingPolicy::default(),
        segments: [B3Hash::digest(b"seg"); 4],
        chat: ChatRequest::empty("duty", 128),
    }
}
