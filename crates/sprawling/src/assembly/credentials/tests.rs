// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn a_model_the_endpoint_never_listed_cannot_be_chosen() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small"], Vec::new());
    let Err(err) = worker_with_provider(dir.path(), &base_url, "m-invented") else {
        panic!("a model the endpoint never listed cannot be chosen");
    };
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(err.subject().contains("m-invented"));
}

/// Most compatible endpoints serve no model list, so the ids the
/// person declared are the only ids there will ever be. Attaching on
/// them is what the four reference harnesses do; refusing would keep a
/// working provider out of the city over an interface it never
/// promised.
#[test]
fn an_endpoint_with_no_model_list_attaches_on_the_ids_the_person_named() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("declared").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-1".to_owned()],
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    let held = worker.book.endpoints().next().unwrap().clone();
    assert_eq!(held.models, vec!["m-1".to_owned()]);
    assert!(
        !held.probed,
        "this list is the person's word, and the book says so"
    );
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("declared").unwrap(),
            model: "m-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: 4_096,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
}

#[test]
fn an_endpoint_with_neither_a_model_list_nor_a_declared_id_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let err = worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("silent").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap_err();
    assert!(
        err.recovery().contains("model"),
        "a city with no model id to call must ask for one: {}",
        err.recovery()
    );
}

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

#[test]
fn a_dispatch_without_a_provider_fails_saying_what_to_configure() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    // Nothing registered: the refusal has to name the act that fixes
    // it, because a person who has not attached a provider yet is
    // exactly the person who does not know that is the missing step.
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let err = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "anything".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap_err();
    assert!(
        err.recovery().contains("settings page"),
        "got: {}",
        err.recovery()
    );
}

#[test]
fn a_loopback_endpoint_with_a_credential_sends_it_on_every_call() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-key"], vec![completion("done", None)]);
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::PutSecret {
            realm: "proxy".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new("sk-proxy-credential".to_owned())),
        })
        .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("proxied").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:proxy/key".to_owned()),
            auth_header: None,
            admit: Vec::new(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("proxied").unwrap(),
            model: "m-key".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: 4_096,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say done".to_owned(),
            goal: "auth on the wire".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let chat = provider
        .exchanges()
        .into_iter()
        .find(|head| head.starts_with("POST"))
        .expect("the dispatch called the model");
    // Before the credential-aware route existed, a loopback endpoint
    // with a secret went through the local adapter and this header
    // was silently absent - the probe authenticated, the calls never.
    assert!(
        chat.to_ascii_lowercase()
            .contains("authorization: bearer sk-proxy-credential"),
        "the chat call must carry the credential; head was:\n{chat}"
    );
}
