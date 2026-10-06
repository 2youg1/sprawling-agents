// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which adapter a chosen model gets, and why.
//!
//! The choice reads only the chosen endpoint and a redemption closure,
//! so it lives here, next to the adapters it mints, rather than in the
//! assembly point's credential cluster — one wiring place, not two.

use kernel::AxError;

use crate::endpoint::{ClientShape, Endpoint, EndpointConfig, HeaderValue, Redemption};
use crate::router::Chosen;

/// The adapter for one chosen model.
///
/// Every chosen model gets an [`Endpoint`], a loopback one included:
/// the endpoint decides locality once, when its client is built, and
/// every adapter for one endpoint shares that client; the stream door is the endpoint's, so a local model answers in
/// increments like any other.
pub fn adapter_for(
    chosen: &Chosen<'_>,
    redemption: Redemption,
    dialect_headers: Vec<(String, String)>,
    monotonic: fn() -> std::time::Instant,
) -> Result<Box<dyn kernel::Model + Send>, AxError> {
    let endpoint = chosen.endpoint;
    let tuning = &endpoint.tuning;
    // A person who names a header the dialect also sets replaces it,
    // rather than adding a second line with the same name: two
    // `anthropic-version` headers is a request no provider promises to
    // read the way either of them meant.
    let mut extra_headers: Vec<(String, HeaderValue)> = dialect_headers
        .into_iter()
        .filter(|(name, _)| {
            !tuning
                .extra_headers
                .iter()
                .any(|(given, _)| given.eq_ignore_ascii_case(name))
        })
        // A dialect's own headers are this build's literals rather
        // than a person's entry, so they carry no reference to redeem
        // and need no scan to clear them.
        .map(|(name, value)| (name, HeaderValue::Plain(value)))
        .collect();
    extra_headers.extend(tuning.extra_headers.iter().cloned());
    let ClientShape {
        url,
        proxying,
        timeout_ms,
    } = endpoint.client_shape();
    let endpoint = Endpoint::over(
        chosen.transport,
        EndpointConfig {
            base_url: url,
            dialect: endpoint.dialect,
            model: chosen.entry.id.clone(),
            auth: endpoint.first_auth()?,
            extra_headers,
            overrides: tuning.applied_overrides(),
            timeout_ms,
            stream_idle_timeout_ms: tuning.idle_timeout_ms(),
            pricing: Some(chosen.entry.clone()),
            proxying,
        },
        redemption,
    )?;
    Ok(Box::new(
        endpoint.gated(chosen.transport, monotonic).with_accounts(
            chosen.endpoint.name.clone(),
            chosen.endpoint.tuning.accounts.clone().unwrap_or_default(),
        ),
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::io::{Read as _, Write as _};

    use super::*;
    use crate::endpoint::AuthSpec;
    use crate::provider::registry::ConnectionKind;
    use crate::router::{AttachedEndpoint, EndpointTuning};

    /// A loopback server speaking the OpenAI shape that answers in
    /// frames, and hands back the request it was sent.
    fn loopback_sse_model() -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let mut request = String::new();
            loop {
                let n = socket.read(&mut buf).unwrap();
                request.push_str(&String::from_utf8_lossy(&buf[..n]));
                let Some(head_end) = request.find("\r\n\r\n") else {
                    continue;
                };
                let length = request[..head_end]
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + length {
                    break;
                }
            }
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n",
                )
                .unwrap();
            for frame in [
                serde_json::json!({"choices": [{"delta": {"content": "on "}}]}),
                serde_json::json!({"choices": [{"delta": {"content": "it"}}]}),
                serde_json::json!({"choices": [{"delta": {}, "finish_reason": "stop"}]}),
            ] {
                socket
                    .write_all(format!("data: {frame}\n\n").as_bytes())
                    .unwrap();
            }
            socket.write_all(b"data: [DONE]\n\n").unwrap();
            request
        });
        (format!("http://{addr}/v1"), server)
    }

    /// **A local model streams like any other.** A loopback endpoint
    /// with no credential and no override is the plainest endpoint
    /// there is, and it was the one endpoint whose answer reached the
    /// page in one burst at the end: the request never asked for a
    /// stream, so no delta frame ever crossed the socket.
    #[test]
    fn a_loopback_model_is_asked_for_a_stream_and_answers_in_increments() {
        let (url, server) = loopback_sse_model();
        let endpoint = AttachedEndpoint {
            name: "local".to_owned(),
            base_url: url,
            dialect: ConnectionKind::OpenAiCompat.wire(),
            connection_kind: ConnectionKind::OpenAiCompat,
            auth: AuthSpec::None,
            models: Vec::new(),
            probed: false,
            tuning: EndpointTuning::default(),
        };
        let entry = crate::market::MarketSnapshot::builtin()
            .unwrap()
            .lookup("local")
            .unwrap()
            .clone();
        let transport = crate::endpoint::Transport::default();
        let chosen = Chosen {
            endpoint: &endpoint,
            entry: &entry,
            transport: &transport,
        };
        let mut model = adapter_for(
            &chosen,
            crate::endpoint::redemption::redemption(),
            Vec::new(),
            crate::endpoint::fakes::monotonic,
        )
        .unwrap();

        let said = std::cell::RefCell::new(Vec::new());
        let mut onto = |held: &kernel::Increment| {
            if let kernel::Increment::Said(text) = held {
                said.borrow_mut().push(text.clone());
            }
        };
        let ret = model
            .call_streaming(&crate::endpoint::fakes::request(), &mut onto)
            .unwrap();

        let request = server.join().unwrap();
        let body: serde_json::Value =
            serde_json::from_str(&request[request.find("\r\n\r\n").unwrap() + 4..]).unwrap();
        assert_eq!(body["stream"], serde_json::Value::Bool(true), "{body}");
        assert_eq!(said.borrow().as_slice(), ["on ", "it"]);
        assert_eq!(ret.stop, Some(kernel::StopReason::EndTurn));
    }

    /// A loopback server that keeps each connection open, answers
    /// `calls` requests in all, and hands back how many connections
    /// they arrived on.
    fn keep_alive_model(calls: usize) -> (String, std::thread::JoinHandle<usize>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })
        .to_string();
        let server = std::thread::spawn(move || {
            let (mut served, mut connections) = (0, 0);
            while served < calls {
                let (mut socket, _) = listener.accept().unwrap();
                connections += 1;
                let mut pending = String::new();
                let mut buf = vec![0u8; 65536];
                while served < calls {
                    let n = socket.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                    let Some(head_end) = pending.find("\r\n\r\n") else {
                        continue;
                    };
                    let length = pending[..head_end]
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if pending.len() < head_end + 4 + length {
                        continue;
                    }
                    pending.clear();
                    served += 1;
                    socket
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                                body.len()
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                }
            }
            connections
        });
        (format!("http://{addr}/v1"), server)
    }

    /// **One endpoint, one client.** Every blocking client starts its
    /// own `reqwest-internal-sync-runtime` thread and keeps its own
    /// connection pool, so four runs on one endpoint that each built a
    /// client held four threads and opened four connections. The calls
    /// here run one after another, so a shared client answers all four
    /// on the one connection its pool kept.
    #[test]
    fn every_adapter_for_one_endpoint_shares_its_client() {
        let (url, server) = keep_alive_model(4);
        let endpoint = AttachedEndpoint {
            name: "local".to_owned(),
            base_url: url,
            dialect: ConnectionKind::OpenAiCompat.wire(),
            connection_kind: ConnectionKind::OpenAiCompat,
            auth: AuthSpec::None,
            models: Vec::new(),
            probed: false,
            tuning: EndpointTuning::default(),
        };
        let entry = crate::market::MarketSnapshot::builtin()
            .unwrap()
            .lookup("local")
            .unwrap()
            .clone();
        let mut book = crate::router::EndpointBook::new();
        book.apply_payload(
            kernel::EventKind::EndpointAttached,
            &crate::router::attached_payload(&endpoint).unwrap(),
        )
        .unwrap();
        book.apply_payload(
            kernel::EventKind::ModelSelected,
            &crate::router::selected_payload(kernel::ModelTag::Main, "local", &entry, None)
                .unwrap(),
        )
        .unwrap();
        for _ in 0..4 {
            let chosen = book
                .select(kernel::ModelTag::Main, &kernel::BuildingPolicy::default())
                .unwrap();
            adapter_for(
                &chosen,
                crate::endpoint::redemption::redemption(),
                Vec::new(),
                crate::endpoint::fakes::monotonic,
            )
            .unwrap()
            .call(&crate::endpoint::fakes::request())
            .unwrap();
        }
        assert_eq!(
            server.join().unwrap(),
            1,
            "connections the four calls opened"
        );
    }
}
