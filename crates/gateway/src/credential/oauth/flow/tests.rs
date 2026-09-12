// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
use super::super::super::custodian::Custodian;
use super::*;
use kernel::SecretRef;
fn sample_token() -> String {
    // Runtime-assembled: the repository never holds a complete
    // high-entropy literal at rest (xtask secret discipline).
    ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat()
}

/// A token endpoint that answers one POST with `status` and `body`,
/// and reports what it was sent.
/// A token endpoint that answers one POST with `status` and `body`,
/// and reports what it was sent.
fn token_endpoint(status: u16, body: String) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).unwrap();
        let seen = String::from_utf8_lossy(&buf[..n]).into_owned();
        let head = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
        seen
    });
    (format!("http://{addr}/v1/oauth/token"), handle)
}
fn profile_at(token_url: &str) -> crate::oauth_profiles::OauthProfile {
    let mut profile = *crate::oauth_profiles::profile("anthropic").unwrap();
    profile.token_endpoint = Box::leak(token_url.to_owned().into_boxed_str());
    profile
}

#[test]
fn a_redeemed_code_becomes_tokens_that_stay_wrapped() {
    let refresh = ["rt-", "K3nD7pQ2", "mX8vB4tL"].concat();
    let body = serde_json::json!({
        "access_token": sample_token(),
        "refresh_token": refresh,
        "expires_in": 3600,
    })
    .to_string();
    let (url, server) = token_endpoint(200, body);
    let profile = profile_at(&url);
    let pending = oauth_begin(
        &profile,
        "v".repeat(64),
        "state-of-its-own-randomness".to_owned(),
    )
    .unwrap();
    let tokens = oauth_redeem(&profile, &pending, "the-code", 5_000).unwrap();
    assert_eq!(tokens.access.as_str(), sample_token());
    assert!(tokens.refresh.is_some());
    assert_eq!(tokens.expires_in_s, Some(3600));

    let sent = server.join().unwrap();
    assert!(sent.contains("the-code"), "the code travels in the body");
    assert!(
        sent.contains(&"v".repeat(64)),
        "and so does the verifier that proves this client asked"
    );
}

#[test]
fn a_refresh_exchanges_the_stored_token_for_a_fresh_pair() {
    let access = ["sk-ant-oat01-", "N7pQ2mK4", "vB1nC5tR"].concat();
    let body = serde_json::json!({ "access_token": access, "expires_in": 3600 }).to_string();
    let (url, server) = token_endpoint(200, body);
    let profile = profile_at(&url);
    let held = Sealed::new(Box::new("the-refresh-token".to_owned()));
    let tokens = oauth_refresh(&profile, &held, 5_000).unwrap();
    assert_eq!(tokens.expires_in_s, Some(3600));
    assert!(
        tokens.refresh.is_none(),
        "a provider that issues no new refresh token is a fact, not a failure"
    );
    let sent = server.join().unwrap();
    assert!(sent.contains("refresh_token"));
    assert!(sent.contains("the-refresh-token"));
}

#[test]
fn a_refused_redeem_says_what_to_do_without_quoting_the_answer() {
    let (url, server) = token_endpoint(
        400,
        serde_json::json!({ "error": "invalid_grant", "code": "the-code" }).to_string(),
    );
    let profile = profile_at(&url);
    let pending = oauth_begin(&profile, "v".repeat(64), "state-value".to_owned()).unwrap();
    let err = oauth_redeem(&profile, &pending, "the-code", 5_000).unwrap_err();
    assert_eq!(err.code(), &AxCode::Provider);
    assert!(err.subject().contains("400"));
    assert!(
        !err.subject().contains("the-code") && !err.recovery().contains("the-code"),
        "a refusal must not carry the code back out: {} / {}",
        err.subject(),
        err.recovery()
    );
    assert!(err.recovery().contains("start the login again"));
    let _ = server.join();
}

#[test]
fn a13_redeemed_value_reaches_the_wire_verbatim() {
    use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig};
    use kernel::DialectKind;
    use kernel::{B3Hash, BuildingPolicy, ChatRequest, Model, ModelRequest};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // Capture the pasted token, then let the endpoint redeem it.
    let mut custodian = Custodian::in_memory();
    let paste = format!("key: {}", sample_token());
    custodian.capture(paste.as_bytes(), "paste").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).unwrap();
        let seen = String::from_utf8_lossy(&buf[..n]).into_owned();
        let body = serde_json::json!({
            "content": [], "stop_reason": "end_turn",
            "usage": { "input_tokens": 0, "output_tokens": 0 },
        })
        .to_string();
        let head = format!(
            "HTTP/1.1 200 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
        seen
    });
    let custodian = std::sync::Arc::new(std::sync::Mutex::new(custodian));
    let resolver_handle = std::sync::Arc::clone(&custodian);
    let mut endpoint = Endpoint::new(
        EndpointConfig {
            base_url: format!("http://{addr}/v1/messages"),
            dialect: DialectKind::Anthropic,
            model: "m".to_owned(),
            auth: AuthSpec::Header {
                name: "x-api-key".to_owned(),
                value: SecretRef::parse("secret:anthropic/cap-1").unwrap(),
            },
            extra_headers: vec![],
            overrides: vec![],
            timeout_ms: 5_000,
            stream_deadline_ms: None,
            pricing: None,
        },
        crate::endpoint::Redemption::without_images(Box::new(move |reference| {
            resolver_handle
                .lock()
                .map_err(|_| {
                    AxError::failure(AxCode::CredentialMissing, "resolve credential", "lock")
                })?
                .resolve(reference)
        })),
    )
    .unwrap();
    endpoint
        .call(&ModelRequest {
            policy: BuildingPolicy::default(),
            segments: [B3Hash::digest(b"s"); 4],
            chat: ChatRequest::empty("m", kernel::Ceiling::new(8).unwrap()),
        })
        .unwrap();
    let seen = server.join().unwrap();
    assert!(
        seen.contains(&format!("x-api-key: {}", sample_token())),
        "the vaulted value reaches the wire verbatim"
    );
}
