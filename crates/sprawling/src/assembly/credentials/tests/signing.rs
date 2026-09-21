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
        family: gateway::Family::ClaudeCode,
        provider: "anthropic",
        api_base: Box::leak(base.clone().into_boxed_str()),
        auth_endpoint: "https://example.invalid/oauth/authorize",
        token_endpoint: Box::leak(format!("{base}/v1/oauth/token").into_boxed_str()),
        scopes: &["user:inference"],
        client_id: "test-client",
        grant: gateway::Grant::AuthorizationCode {
            redirect_uri: "https://example.invalid/callback",
        },
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

    // Every provider this build knows is named in the refusal, and
    // the sentence is the table's rather than this call site's.
    for row in gateway::OAUTH_PROFILES {
        assert!(err.recovery().contains(row.provider), "{}", row.provider);
    }

    // OpenAI's row states every fact a login needs, so the login
    // begins.
    worker
        .handle(channels::Command::Login {
            provider: channels::ProviderName::parse("openai").unwrap(),
            step: channels::LoginStep::Begin,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"login"),
        })
        .unwrap();
}

/// Which face a finished login attaches on is the family's statement,
/// read through the connection the registration is. The Codex
/// subscription answers on the responses face, and a second mapping
/// from the provider word `openai` to a dialect used to attach it on
/// the chat face — where its first call is a 404.
#[test]
fn a_subscription_is_attached_on_the_face_its_family_answers_on() {
    for row in gateway::OAUTH_PROFILES {
        let wire = gateway::ConnectionKind::Harness(row.family).wire();
        let expected = match row.family {
            gateway::Family::Codex => kernel::DialectKind::OpenAiResponses,
            gateway::Family::ClaudeCode => kernel::DialectKind::Anthropic,
            gateway::Family::GrokBuild | gateway::Family::KimiCli => kernel::DialectKind::OpenAi,
        };
        assert_eq!(wire, expected, "{}", row.provider);
    }
}

/// A vendor that signs in on a second device: it issues a code, hands
/// over the tokens, then answers the model list the attach probes
/// for. Dispatched on the request line, so the order the flow makes
/// them in is part of what this asserts.
#[cfg(test)]
fn fake_device_provider() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let issued = serde_json::json!({
            "device_code": "dev-7f2a",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://auth.example.invalid/device",
            // Zero, which this city floors to one second and then
            // waits out; the test spends it rather than pretending
            // the rule is not there.
            "interval": 0,
            "expires_in": 1800,
        })
        .to_string();
        // Assembled at runtime: a credential-shaped literal is what
        // the secret gate keeps out of the repository.
        let access = ["xai-", "Qz7mK2pL9vB4nC5"].concat();
        let refresh = ["xai-rt-", "Rt3nD8qX2vC6mB1"].concat();
        let tokens = serde_json::json!({
            "access_token": access,
            "refresh_token": refresh,
            "expires_in": 3600,
        })
        .to_string();
        let models = serde_json::json!({ "data": [{ "id": "grok-4" }] }).to_string();
        for _ in 0..3 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = vec![0u8; 65536];
            let Ok(n) = stream.read(&mut buf) else { break };
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            let body = if request.starts_with("POST /device/code") {
                issued.clone()
            } else if request.starts_with("POST /token") {
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

/// The device-code half of the four subscriptions, end to end: the
/// code the person reads reaches the ledger, the code they type back
/// is checked against it, the RFC's interval is waited out, and the
/// endpoint the login was for is attached.
#[test]
fn a_device_code_login_shows_a_code_waits_its_interval_and_attaches_the_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base, server) = fake_device_provider();
    let profile = gateway::OauthProfile {
        family: gateway::Family::GrokBuild,
        provider: "xai",
        api_base: Box::leak(base.clone().into_boxed_str()),
        auth_endpoint: "",
        token_endpoint: Box::leak(format!("{base}/token").into_boxed_str()),
        scopes: &["api:access"],
        client_id: "test-client",
        grant: gateway::Grant::DeviceCode {
            authorization_endpoint: Box::leak(format!("{base}/device/code").into_boxed_str()),
        },
        headers: &[],
    };
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    worker
        .login_with(&profile, "xai", channels::LoginStep::Begin)
        .unwrap();

    // Another login's code finishes nothing, and no byte is sent.
    let wrong = worker
        .login_with(
            &profile,
            "xai",
            channels::LoginStep::Code {
                code: "AAAA-BBBB".to_owned(),
            },
        )
        .unwrap_err();
    assert_eq!(wrong.code(), &AxCode::InvalidArgs);

    // Inside the interval the vendor is not asked at all; the person
    // is told how long to leave it.
    let early = worker
        .login_with(
            &profile,
            "xai",
            channels::LoginStep::Code {
                code: "WDJB-MJHT".to_owned(),
            },
        )
        .unwrap_err();
    assert_eq!(early.code(), &AxCode::CredentialMissing);
    assert!(early.recovery().contains("seconds between tries"));

    std::thread::sleep(std::time::Duration::from_millis(1_100));
    worker
        .login_with(
            &profile,
            "xai",
            channels::LoginStep::Code {
                code: "WDJB-MJHT".to_owned(),
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
    assert!(
        history.contains("WDJB-MJHT") && history.contains("auth.example.invalid/device"),
        "a person reads the code and the page out of the history the page folds"
    );
    assert!(
        !history.contains("dev-7f2a"),
        "the device code is the secret half and never reaches the ledger"
    );
    assert!(history.contains("secret:xai/oauth"));
    assert!(history.contains("endpoint_attached"));

    let sent = server.join().unwrap();
    assert_eq!(sent.len(), 3, "issue, redeem, then the attach probe");
    assert!(
        sent.iter().any(|request| request
            .contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code")),
        "the token ask carries the grant RFC 8628 names"
    );
    assert!(
        !history.contains("xai-Qz7mK2p"),
        "a token never reaches the ledger"
    );
}
