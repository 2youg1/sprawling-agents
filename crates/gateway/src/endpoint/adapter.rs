// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which adapter a chosen model gets, and why.
//!
//! The assembly point used to own this choice as `RunWorker::adapter_for`:
//! a credential cluster holding an assembly line. The choice reads only
//! the chosen endpoint and a redemption closure, so it lives here, next
//! to the adapters it mints — one wiring place, not two.

use kernel::AxError;

use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig, HeaderValue, Redemption};
use crate::router::Chosen;

/// The adapter for one chosen model.
///
/// A loopback endpoint speaking the OpenAI shape goes through the
/// local adapter, which is loopback-only by construction; everything
/// else goes through the general one, which refuses to carry a
/// confidential building's bytes off this machine.
///
/// A credential decides the route too: the local adapter has no
/// authentication surface, so a loopback endpoint that was attached
/// with a secret (a local proxy, LiteLLM, a corporate gateway) must
/// take the general path. So does a header or a body override the
/// person set: the local adapter can send neither, and an adapter that
/// quietly dropped them would call the endpoint a different way from
/// the one the form showed.
pub fn adapter_for(
    chosen: &Chosen<'_>,
    redemption: Redemption,
    dialect_headers: Vec<(String, String)>,
) -> Result<Box<dyn kernel::Model + Send>, AxError> {
    let endpoint = chosen.endpoint;
    let tuning = &endpoint.tuning;
    // The tuning answers this, because the figure an untuned endpoint
    // is called with is the tuning's own default and is stated there.
    let timeout_ms = tuning.call_timeout_ms();
    if endpoint.is_local()
        && matches!(endpoint.dialect, kernel::DialectKind::OpenAi)
        && matches!(endpoint.auth, AuthSpec::None)
        && tuning.is_plain()
    {
        let native = crate::Native::new(crate::NativeConfig {
            base_url: endpoint.chat_url(),
            model: chosen.entry.id.clone(),
            timeout_ms,
            pricing: Some(chosen.entry.clone()),
            proxying: tuning.proxying,
        })?;
        return Ok(Box::new(native));
    }
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
    let endpoint = Endpoint::new(
        EndpointConfig {
            base_url: endpoint.chat_url(),
            dialect: endpoint.dialect,
            model: chosen.entry.id.clone(),
            auth: endpoint.auth.clone(),
            extra_headers,
            overrides: tuning.applied_overrides(),
            timeout_ms,
            stream_idle_timeout_ms: tuning.idle_timeout_ms(),
            pricing: Some(chosen.entry.clone()),
            proxying: tuning.proxying,
        },
        redemption,
    )?;
    Ok(Box::new(endpoint))
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
        let chosen = Chosen {
            endpoint: &endpoint,
            entry: &entry,
        };
        let mut model = adapter_for(
            &chosen,
            crate::endpoint::redemption::redemption(),
            Vec::new(),
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
}
