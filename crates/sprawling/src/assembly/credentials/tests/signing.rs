// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a credential enters this city: enrolment, and a subscription
//! login carried through to an endpoint.

use super::super::*;
use crate::assembly::*;

#[test]
fn an_enrolled_credential_leaves_only_a_reference_in_the_history() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    // Assembled at runtime: a credential-shaped literal is what the
    // secret gate keeps out of the repository.
    let token = ["sk-live-", "9f2c4a7e1b8d"].concat();
    worker
        .handle(channels::Command::PutSecret {
            realm: "house".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new(token.clone())),
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("secret:house/key"));
    assert!(
        !history.contains(&token),
        "the ledger records where a credential lives, never what it is"
    );
    // And it is redeemable afterwards, which is the other half: a
    // vault that records the act without keeping the value would
    // fail later, far from here.
    let resolver = worker.resolver();
    let reference = kernel::SecretRef::parse("secret:house/key").unwrap();
    let redeemed = resolver(&reference).unwrap().into_vault_value();
    assert_eq!(redeemed.as_str(), token);
}

/// A provider that answers the two requests a finished login makes:
/// the token POST, then the model list the attach probes for.
#[cfg(test)]
fn fake_oauth_provider() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let access = ["sk-ant-oat01-", "Qz7mK2p", "L9vB4nC5"].concat();
        let refresh = ["sk-ant-ort01-", "Rt3nD8q", "X2vC6mB1"].concat();
        let tokens = serde_json::json!({
            "access_token": access,
            "refresh_token": refresh,
            "expires_in": 3600,
        })
        .to_string();
        let models = serde_json::json!({ "data": [{ "id": "claude-sonnet-4-6" }] }).to_string();
        for _ in 0..2 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = vec![0u8; 65536];
            let Ok(n) = stream.read(&mut buf) else { break };
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = if request.starts_with("POST") {
                tokens.clone()
            } else {
                models.clone()
            };
            seen.push(request);
            let head = format!(
                "HTTP/1.1 200 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body.as_bytes());
        }
        seen
    });
    (format!("http://{addr}"), handle)
}

#[test]
fn a_subscription_login_ends_with_a_credential_in_the_vault_and_an_endpoint_attached() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base, server) = fake_oauth_provider();
    let profile = gateway::OauthProfile {
        provider: "anthropic",
        api_base: Box::leak(base.clone().into_boxed_str()),
        auth_endpoint: "https://example.invalid/oauth/authorize",
        token_endpoint: Box::leak(format!("{base}/v1/oauth/token").into_boxed_str()),
        scopes: &["user:inference"],
        client_id: "test-client",
        redirect_uri: "https://example.invalid/callback",
        headers: &[],
    };
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    // A code with no request behind it proves nothing, and is
    // refused before any byte is sent.
    let err = worker
        .login_with(
            &profile,
            "anthropic",
            channels::LoginStep::Code {
                code: "x".to_owned(),
            },
        )
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::CredentialMissing);
    assert!(err.recovery().contains("start the login first"));

    worker
        .login_with(&profile, "anthropic", channels::LoginStep::Begin)
        .unwrap();
    worker
        .login_with(
            &profile,
            "anthropic",
            channels::LoginStep::Code {
                code: "the-code".to_owned(),
            },
        )
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("login_started"));
    assert!(
        history.contains("code_challenge_method=S256"),
        "the url a person is asked to open is the one history recorded"
    );
    assert!(
        history.contains("secret:anthropic/oauth"),
        "the credential is a reference in history, never a value"
    );
    assert!(history.contains("endpoint_attached"));

    let sent = server.join().unwrap();
    assert!(
        !history.contains("sk-ant-oat01-"),
        "a token never reaches the ledger"
    );
    assert!(
        sent.iter().any(|request| request.contains("the-code")),
        "the code was redeemed against the provider"
    );
    assert!(
        sent.iter().any(|request| request
            .to_ascii_lowercase()
            .contains("authorization: bearer")),
        "and the attach carries the credential the login just earned"
    );
}

#[test]
fn a_login_for_a_provider_this_build_has_no_flow_for_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let err = worker
        .handle(channels::Command::Login {
            provider: channels::ProviderName::parse("modelscope").unwrap(),
            step: channels::LoginStep::Begin,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"login"),
        })
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("API key"));

    // The one whose intelligence row is empty fails closed rather
    // than sending a person to an empty URL.
    let err = worker
        .handle(channels::Command::Login {
            provider: channels::ProviderName::parse("openai").unwrap(),
            step: channels::LoginStep::Begin,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"login"),
        })
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("intelligence incomplete"));
}
