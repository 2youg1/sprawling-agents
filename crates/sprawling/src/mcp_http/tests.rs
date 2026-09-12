// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use protocol::Outbound as _;

/// A vault holding one credential, so a configured header that names
/// one can be redeemed the way a real building's would be.
fn vault() -> gateway::SecretResolver {
    Box::new(|reference: &kernel::SecretRef| {
        Ok(kernel::Sealed::new(Box::new(format!(
            "held-{}",
            reference.name()
        ))))
    })
}

/// A server that answers one POST with `status` and `body`.
#[cfg(test)]
fn fake_server(status: u16, body: String) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read as _, Write as _};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let read = stream.read(&mut buf).unwrap();
        let seen = String::from_utf8_lossy(&buf[..read]).into_owned();
        let head = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
        seen
    });
    (format!("http://{addr}/mcp"), handle)
}

#[test]
fn a_hosted_server_answers_and_the_configured_header_travels() {
    let (url, server) = fake_server(
        200,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}".to_owned(),
    );
    let mut held = HttpServer::open(
        &url,
        &[("X-Desk-Key".to_owned(), "opaque-value".to_owned())],
        &vault(),
    )
    .unwrap();
    let answer = held
        .call(
            &protocol::Rpc::new().list_tools(),
            protocol::EXTERNAL_CALL_PATIENCE,
        )
        .unwrap();
    assert!(protocol::Rpc::read(&answer).is_ok());
    let sent = server.join().unwrap().to_ascii_lowercase();
    assert!(sent.contains("x-desk-key: opaque-value"));
    assert!(sent.contains("accept: application/json, text/event-stream"));
}

#[test]
fn an_answer_that_arrives_as_a_stream_is_read_as_one_message() {
    assert_eq!(
        one_message("event: message\ndata: {\"result\":{}}\n\n").as_deref(),
        Some("{\"result\":{}}")
    );
    assert_eq!(
        one_message(" {\"result\":{}} ").as_deref(),
        Some("{\"result\":{}}")
    );
    assert!(
        one_message("event: ping\n\n").is_none(),
        "a stream carrying no message is refused rather than joined into one"
    );
}

#[test]
fn a_refusing_server_states_the_status_without_quoting_its_page() {
    let (url, server) = fake_server(502, "{\"error\":\"account suspended\"}".to_owned());
    let mut held = HttpServer::open(&url, &[], &vault()).unwrap();
    let err = held
        .call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ToolUnavailable);
    assert!(err.subject().contains("502"));
    assert!(!err.subject().contains("suspended"));
    let _ = server.join();
}

/// A server that answers 401 or 403 is up and understood the request,
/// and what it wants is an account. That is a different thing for a
/// person to do than checking the address, so it is a different code -
/// and the health view reads it as `authenticating` rather than as a
/// failure.
#[test]
fn a_server_wanting_an_account_refuses_with_the_credential_code() {
    let (url, server) = fake_server(401, "{\"error\":\"sign in\"}".to_owned());
    let mut held = HttpServer::open(&url, &[], &vault()).unwrap();
    let err = held
        .call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::CredentialMissing);
    assert!(err.recovery().contains("vault"), "{}", err.recovery());
    let _ = server.join();
}

/// A server that answers `rounds` requests, each with `body`, and
/// hands out `session` on the first. Returns everything it was sent.
fn sessioned_server(
    rounds: usize,
    session: &'static str,
    body: &'static str,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for round in 0..rounds {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let read = stream.read(&mut buf).unwrap();
            seen.push(String::from_utf8_lossy(&buf[..read]).into_owned());
            let handed = if round == 0 {
                format!("mcp-session-id: {session}\r\n")
            } else {
                String::new()
            };
            let head = format!(
                "HTTP/1.1 200 X\r\ncontent-type: application/json\r\n{handed}\
                 content-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(body.as_bytes()).unwrap();
        }
        seen
    });
    (format!("http://{addr}/mcp"), handle)
}

/// The whole reason this card exists: a session id handed out at
/// initialization has to travel on every later request, and so does
/// the negotiated protocol version. Without them a server that
/// keeps state answers 400 to everything after the handshake.
#[test]
fn a_session_handed_out_at_initialization_travels_on_every_later_request() {
    const OPENED: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\",\"capabilities\":{},\"serverInfo\":{\"name\":\"hosted\",\"version\":\"1\"},\"tools\":[]}}";
    let (url, server) = sessioned_server(3, "session-abc", OPENED);
    let mut held = HttpServer::open(&url, &[], &vault()).unwrap();
    let mut rpc = protocol::Rpc::new();
    let opened =
        protocol::handshake(&mut held, &mut rpc, protocol::EXTERNAL_CALL_PATIENCE).unwrap();
    assert_eq!(opened.protocol_version, "2025-03-26");
    held.call(&rpc.list_tools(), protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap();

    let sent = server.join().unwrap();
    let first = sent[0].to_ascii_lowercase();
    assert!(first.contains("\"method\":\"initialize\""), "{first}");
    assert!(
        !first.contains("mcp-session-id"),
        "there is no session to name before one is handed out"
    );
    for later in &sent[1..] {
        let later = later.to_ascii_lowercase();
        assert!(later.contains("mcp-session-id: session-abc"), "{later}");
        assert!(
            later.contains("mcp-protocol-version: 2025-03-26"),
            "{later}"
        );
    }
    assert!(sent[1].contains("notifications/initialized"));
}

/// A 404 on a request carrying a session id means the server ended
/// the session. The id is dropped so the next handshake opens a new
/// one, which is what the specification asks for; keeping it would
/// make every later request fail the same way for ever.
#[test]
fn a_session_the_server_ended_is_forgotten_rather_than_kept() {
    let (url, server) = fake_server(404, "{}".to_owned());
    let mut held = HttpServer::open(&url, &[], &vault()).unwrap();
    if let Ok(mut session) = held.session.lock() {
        session.id = Some("stale".to_owned());
    }
    let err = held
        .call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap_err();
    assert!(err.subject().contains("ended this session"));
    assert_eq!(
        held.session.lock().unwrap().id,
        None,
        "a session the server disowned is not sent back to it"
    );
    let _ = server.join();
}

/// A paid server's key belongs in the vault, not in a building's
/// `CONFIG.toml`. `xtask secret` cannot see a city's configuration,
/// because a city is not in this repository - so nothing but this
/// would have caught a key written out there in plaintext.
#[test]
fn a_header_naming_a_credential_carries_the_key_and_never_the_reference() {
    let (url, server) = fake_server(
        200,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}".to_owned(),
    );
    let mut held = HttpServer::open(
        &url,
        &[("X-Api-Key".to_owned(), "secret:exa/api".to_owned())],
        &vault(),
    )
    .unwrap();
    held.call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap();
    let sent = server.join().unwrap();
    assert!(sent.contains("x-api-key: held-api"), "{sent}");
    assert!(
        !sent.contains("secret:"),
        "the reference is redeemed, not forwarded"
    );
}

/// The value is never in `Debug` either: a diagnostic line is the
/// easiest place for a credential to escape to.
#[test]
fn the_debug_face_names_the_header_and_never_its_value() {
    let held = HttpServer::open(
        "http://127.0.0.1:1/mcp",
        &[("X-Api-Key".to_owned(), "secret:exa/api".to_owned())],
        &vault(),
    )
    .unwrap();
    let drawn = format!("{held:?}");
    assert!(drawn.contains("X-Api-Key"));
    assert!(!drawn.contains("held-api"));
}

/// A header with no name is a row somebody started and did not finish,
/// and it is refused where it is written rather than sent as a header
/// the far end cannot read.
#[test]
fn a_header_with_no_name_is_refused_before_any_request() {
    let err = HttpServer::open(
        "http://127.0.0.1:1/mcp",
        &[(" ".to_owned(), "opaque-value".to_owned())],
        &vault(),
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(!err.subject().contains("opaque-value"), "{}", err.subject());
}
