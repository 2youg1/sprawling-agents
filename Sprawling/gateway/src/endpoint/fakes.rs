// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![cfg(test)]
#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::disallowed_methods,
    reason = "test helpers"
)]

pub(crate) fn monotonic() -> std::time::Instant {
    std::time::Instant::now()
}

use std::time::Duration;

use kernel::{B3Hash, BuildingPolicy, ChatRequest, DialectKind, ModelRequest, SecretRef};
use serde_json::Value;

use super::config::{AuthSpec, EndpointConfig};

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

pub(crate) fn answering_provider(
    answer: impl Fn(&str) -> (u16, String) + Send + 'static,
) -> (
    std::net::SocketAddr,
    std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let noted = std::sync::Arc::clone(&heard);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut buf = vec![0u8; 65536];
            let mut request = String::new();
            let body = loop {
                let Ok(n) = stream.read(&mut buf) else {
                    break None;
                };
                if n == 0 {
                    break None;
                }
                request.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(head_end) = request.find("\r\n\r\n") {
                    let content_length = request[..head_end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= head_end + 4 + content_length {
                        break Some(request[head_end + 4..].to_owned());
                    }
                }
            };
            let Some(body) = body else { continue };
            let (status, reply) = answer(&body);
            noted.lock().unwrap().push(body);
            let payload = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
            if stream.write_all(payload.as_bytes()).is_err() {
                continue;
            }
        }
    });
    (addr, heard)
}

#[cfg(test)]
const SAW_OPENING_WAIT: Duration = Duration::from_secs(2);

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test helper"
)]
pub(crate) fn paced_stream_provider(
    frames: Vec<String>,
    gap: Duration,
    then_quiet_for: Duration,
) -> (String, std::thread::JoinHandle<()>) {
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
        let _ = stream.write_all(
            b"HTTP/1.1 200 X\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n",
        );
        let _ = stream.flush();
        for frame in &frames {
            std::thread::sleep(gap);
            let _ = stream.write_all(format!("data: {frame}\n\n").as_bytes());
            let _ = stream.flush();
        }
        std::thread::sleep(then_quiet_for);
        let _ = stream.write_all(b"data: [DONE]\n\n");
        let _ = stream.flush();
        drop(stream);
    });
    (format!("http://{addr}/v1/messages"), handle)
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
pub(crate) fn silent_provider() -> String {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let held = listener.accept();

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
        extra_headers: vec![(
            "anthropic-version".to_owned(),
            crate::endpoint::HeaderValue::Plain("2023-06-01".to_owned()),
        )],
        overrides: vec![(
            "/metadata/user_id".to_owned(),
            Value::String("city".to_owned()),
        )],
        timeout_ms: 5_000,
        stream_idle_timeout_ms: None,
        proxying: kernel::Proxying::default(),
        pricing: Some(
            crate::market::MarketSnapshot::builtin()
                .unwrap()
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
pub(crate) fn request() -> ModelRequest<'static> {
    ModelRequest {
        policy: BuildingPolicy::default(),
        segments: [B3Hash::digest(b"seg"); 4],
        chat: ChatRequest::empty("duty", kernel::Ceiling::new(128).unwrap()),
    }
}
