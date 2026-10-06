// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A hosted server that answers every method it is sent for `requests`
/// requests, and hands back the head of each one in arrival order.
fn recording_server(requests: usize) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::Write as _;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let serving = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..requests {
            let (stream, _) = listener.accept().unwrap();
            let (mut reader, headers, request) = read_http_request(stream);
            let result = match request["method"].as_str().unwrap() {
                "initialize" => serde_json::json!({
                    "protocolVersion": agent_protocols::PROTOCOL_VERSION,
                    "serverInfo": {"name": "hosted"}
                }),
                "tools/list" => serde_json::json!({"tools": [{"name": "ping"}]}),
                _ => serde_json::json!({"content": [{"type": "text", "text": "pong"}]}),
            };
            let body = if request.get("id").is_some() {
                serde_json::to_vec(
                    &serde_json::json!({"jsonrpc": "2.0", "id": request["id"], "result": result}),
                )
                .unwrap()
            } else {
                Vec::new()
            };
            let status = if body.is_empty() { 202 } else { 200 };
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            reader.get_mut().write_all(response.as_bytes()).unwrap();
            reader.get_mut().write_all(&body).unwrap();
            seen.push(format!("{headers}\r\n{request}"));
        }
        seen
    });
    (url, serving)
}

/// One hosted search account: the server at `url`, its key carried in
/// `x-api-key` by `reference`.
fn keyed_server(url: &str, reference: &str) -> kernel::McpServer {
    kernel::McpServer {
        label: kernel::ServerLabel::parse("hosted").unwrap(),
        transport: kernel::McpTransport::Http {
            url: url.to_owned(),
            headers: vec![("x-api-key".to_owned(), reference.to_owned())],
        },
    }
}

fn worker_holding(city_root: &Path, keys: &[(&str, &str)]) -> RunWorker {
    crate::worker::fixture::init_city(city_root).unwrap();
    let mut worker = RunWorker::new(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    for (name, value) in keys {
        worker
            .handle(wire::Command::PutSecret {
                realm: "search".to_owned(),
                name: (*name).to_owned(),
                value: kernel::Sealed::new(Box::new((*value).to_owned())),
            })
            .unwrap();
    }
    worker
}

fn call_once(tools: &[agent_protocols::McpTool]) {
    use kernel::Tool as _;
    tools[0]
        .invoke(&kernel::ToolCall {
            id: "search".to_owned(),
            name: tools[0].meta().name.clone(),
            args: kernel::Payload::new(serde_json::Map::new()).unwrap(),
        })
        .unwrap();
}

/// The two accounts of one supplier differ only in the reference their
/// header carries, and each is its own connection: switching accounts
/// switches connections, so no request of one carries the other's key.
#[test]
fn two_accounts_of_one_server_never_share_a_connection() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &[("alpha", "key-alpha"), ("beta", "key-beta")]);
    let (url, serving) = recording_server(8);
    for name in ["alpha", "beta"] {
        let (tools, reached) = worker
            .connectors
            .connect(
                &keyed_server(&url, &format!("secret:search/{name}")),
                dir.path(),
                false,
                &worker.resolver(),
            )
            .unwrap();
        assert!(
            matches!(reached, Reached::Connected(_)),
            "account {name} opens its own connection"
        );
        call_once(&tools);
    }
    let sent = serving.join().unwrap();
    let keys: Vec<&str> = sent
        .iter()
        .map(|request| {
            if request.contains("key-alpha") {
                "alpha"
            } else if request.contains("key-beta") {
                "beta"
            } else {
                "none"
            }
        })
        .collect();
    assert_eq!(
        keys,
        [["alpha"; 4], ["beta"; 4]].concat(),
        "every request of one account carries that account's key and no other"
    );
}

/// A key stored again under the same reference reaches the next call:
/// the connection that redeemed the old value is dropped when the vault
/// takes the new one, so the old header is never sent again.
#[test]
fn a_key_replaced_through_put_secret_is_never_sent_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = worker_holding(dir.path(), &[("alpha", "key-old")]);
    let (url, serving) = recording_server(8);
    let server = keyed_server(&url, "secret:search/alpha");
    let (tools, _) = worker
        .connectors
        .connect(&server, dir.path(), false, &worker.resolver())
        .unwrap();
    call_once(&tools);

    worker
        .handle(wire::Command::PutSecret {
            realm: "search".to_owned(),
            name: "alpha".to_owned(),
            value: kernel::Sealed::new(Box::new("key-new".to_owned())),
        })
        .unwrap();
    let (tools, reached) = worker
        .connectors
        .connect(&server, dir.path(), false, &worker.resolver())
        .unwrap();
    assert!(
        matches!(reached, Reached::Connected(_)),
        "a replaced key opens a new connection"
    );
    call_once(&tools);
    drop(tools);

    let sent = serving.join().unwrap();
    assert!(
        sent[4..].iter().all(|request| request.contains("key-new")),
        "every request after the key was replaced carries the new key"
    );
    assert!(
        sent[4..].iter().all(|request| !request.contains("key-old")),
        "and none of them the old one"
    );
}
