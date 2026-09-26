// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::Outbound as _;
use std::io::{Read as _, Write as _};

fn vault() -> gateway::SecretResolver {
    Box::new(|reference: &kernel::SecretRef| {
        Ok(kernel::Sealed::new(Box::new(format!(
            "held-{}",
            reference.name()
        ))))
    })
}

/// The announcement's deadline bounds opening the stream, not the
/// stream: a server that stays quiet for longer than that deadline
/// after announcing is still answered on the same stream.
#[test]
fn a_stream_outlives_the_deadline_it_was_opened_within() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let answer = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}";
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        assert!(stream.read(&mut buf).unwrap() > 0);
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\nevent: endpoint\ndata: /messages\n\n",
            )
            .unwrap();
        stream.flush().unwrap();
        let (mut post, _) = listener.accept().unwrap();
        assert!(post.read(&mut buf).unwrap() > 0);
        post.write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .unwrap();
        std::thread::sleep(Duration::from_millis(600));
        stream
            .write_all(format!("data: {answer}\n\n").as_bytes())
            .unwrap();
        stream.flush().unwrap();
    });
    let mut held = SseServer::open_within(
        &format!("http://{addr}/sse"),
        &[],
        &vault(),
        Duration::from_millis(200),
    )
    .unwrap();
    let answered = held.call(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}",
        TimeoutMs(5_000),
    );
    server.join().unwrap();
    assert_eq!(answered.unwrap(), answer);
}

/// A path is what a server behind a proxy announces, and it is
/// resolved against the address this city actually reached.
#[test]
fn an_announced_path_is_resolved_against_the_streams_own_address() {
    assert_eq!(
        resolved_against("https://example.test/sse", "/messages?sessionId=7").unwrap(),
        "https://example.test/messages?sessionId=7"
    );
}

/// A whole url is taken as it stands: a server that names another
/// host means that host.
#[test]
fn an_announced_url_is_taken_as_it_stands() {
    assert_eq!(
        resolved_against("https://example.test/sse", "https://other.test/messages").unwrap(),
        "https://other.test/messages"
    );
}

/// A server that answers 401 is up, understood the request and wants
/// an account. That is a different answer from a server that is
/// down, and the code says so.
#[test]
fn a_server_wanting_an_account_refuses_with_the_credential_code() {
    assert_eq!(
        refused("https://example.test/sse", 401).code(),
        &AxCode::CredentialMissing
    );
    assert_eq!(
        refused("https://example.test/sse", 502).code(),
        &AxCode::ToolUnavailable
    );
}
