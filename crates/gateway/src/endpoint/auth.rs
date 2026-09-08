// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which header a credential travels in — a fact about the compatible
//! format, not about the person filling in a form.
//!
//! Anthropic wants an API key in `x-api-key`; `Authorization: Bearer`
//! there carries a short-lived federation token only, so a key sent
//! that way is answered 401. The OpenAI-compatible shape wants
//! `Bearer`. One constructor holds both, because a registration page
//! that chose for itself would be a second authority on a question the
//! compatible format already answers.

use kernel::{DialectKind, SecretRef};

use super::config::AuthSpec;

impl AuthSpec {
    /// The credential header for one compatible format.
    ///
    /// `header` is what the person typed, and it always wins: a
    /// third-party endpoint may want a name neither first party uses,
    /// and this build cannot know them all.
    #[must_use]
    pub fn for_dialect(
        dialect: DialectKind,
        reference: SecretRef,
        header: Option<String>,
    ) -> AuthSpec {
        match (header, dialect) {
            (Some(name), _) => AuthSpec::Header {
                name,
                value: reference,
            },
            (None, DialectKind::Anthropic) => AuthSpec::Header {
                name: ANTHROPIC_KEY_HEADER.to_owned(),
                value: reference,
            },
            // Anything not spelled here is served by the
            // OpenAI-compatible shape, which is where `Bearer` comes
            // from.
            (None, _) => AuthSpec::Bearer(reference),
        }
    }
}

/// The header Anthropic's own documentation prints for an API key.
/// <https://platform.claude.com/docs/en/api/messages>
const ANTHROPIC_KEY_HEADER: &str = "x-api-key";

/// A provider that answers a model list to `x-api-key` and 401 to
/// `Authorization: Bearer` — which is what the first party and the
/// endpoints written against it actually do.
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
fn key_only_provider() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let list = serde_json::json!({ "data": [{ "id": "m-1" }] }).to_string();
        let Ok((mut stream, _)) = listener.accept() else {
            return seen;
        };
        let mut buf = vec![0u8; 65536];
        let Ok(n) = stream.read(&mut buf) else {
            return seen;
        };
        let request = String::from_utf8_lossy(&buf[..n]).into_owned();
        let lower = request.to_ascii_lowercase();
        let (status, body) = if lower.contains("x-api-key:") {
            (200, list)
        } else {
            (401, r#"{"error":"authentication_error"}"#.to_owned())
        };
        seen.push(request);
        let head = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body.as_bytes());
        seen
    });
    (format!("http://{addr}/v1"), handle)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::endpoint::config::{Endpoint, EndpointConfig, config};
    use crate::endpoint::redemption::redemption;

    fn reference() -> SecretRef {
        SecretRef::parse("secret:anthropic/api").unwrap()
    }

    #[test]
    fn an_anthropic_key_reaches_the_provider_in_the_header_it_accepts() {
        let (url, handle) = key_only_provider();
        let endpoint = Endpoint::new(
            EndpointConfig {
                auth: AuthSpec::for_dialect(DialectKind::Anthropic, reference(), None),
                ..config(&url)
            },
            redemption(),
        )
        .unwrap();
        let ids = endpoint.list_models(&url).unwrap();
        assert_eq!(ids, vec!["m-1".to_owned()]);
        let seen = handle.join().unwrap();
        assert!(
            seen[0].to_ascii_lowercase().contains("x-api-key: sk-test-"),
            "the key travels in x-api-key, never as a bearer token: {}",
            seen[0]
        );
    }

    #[test]
    fn the_header_the_person_named_outranks_the_compatible_format() {
        let auth = AuthSpec::for_dialect(
            DialectKind::Anthropic,
            reference(),
            Some("x-goog-api-key".to_owned()),
        );
        match auth {
            AuthSpec::Header { name, .. } => assert_eq!(name, "x-goog-api-key"),
            other => panic!("an explicit header name always wins: {other:?}"),
        }
    }

    #[test]
    fn the_openai_shape_keeps_the_bearer_header() {
        let auth = AuthSpec::for_dialect(DialectKind::OpenAi, reference(), None);
        assert!(matches!(auth, AuthSpec::Bearer(_)));
    }
}
