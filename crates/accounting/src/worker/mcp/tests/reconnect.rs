// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn session_ending_server() -> (kernel::McpServer, std::thread::JoinHandle<Vec<String>>) {
    use std::io::Write as _;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let serving = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (method, status, session, body) in [
            (
                "initialize",
                200,
                "old",
                serde_json::json!({"protocolVersion": "2025-03-26", "serverInfo": {"name": "hosted"}}),
            ),
            ("notifications/initialized", 202, "", serde_json::json!({})),
            (
                "tools/list",
                200,
                "",
                serde_json::json!({"tools": [{"name": "ping"}]}),
            ),
            ("tools/call", 404, "", serde_json::json!({})),
            (
                "initialize",
                200,
                "new",
                serde_json::json!({"protocolVersion": agent_protocols::PROTOCOL_VERSION, "serverInfo": {"name": "hosted"}}),
            ),
            ("notifications/initialized", 202, "", serde_json::json!({})),
            (
                "tools/list",
                200,
                "",
                serde_json::json!({"tools": [{"name": "pong"}]}),
            ),
            ("tools/call", 200, "", serde_json::json!({})),
        ] {
            let (stream, _) = listener.accept().unwrap();
            let (mut reader, headers, request) = read_http_request(stream);
            assert_eq!(request["method"], method);
            seen.push(format!("{headers}\r\n{request}"));
            let body = if seen.len() == 8 {
                vec![0xff]
            } else if status == 202 {
                Vec::new()
            } else {
                serde_json::to_vec(
                    &serde_json::json!({"jsonrpc": "2.0", "id": request["id"], "result": body}),
                )
                .unwrap()
            };
            let handed = if session.is_empty() {
                String::new()
            } else {
                format!("mcp-session-id: {session}\r\n")
            };
            let response = format!(
                "HTTP/1.1 {status} X\r\n{handed}content-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            reader.get_mut().write_all(response.as_bytes()).unwrap();
            reader.get_mut().write_all(&body).unwrap();
        }
        seen
    });
    (
        kernel::McpServer {
            label: kernel::ServerLabel::parse("hosted").unwrap(),
            transport: kernel::McpTransport::Http {
                url,
                headers: Vec::new(),
            },
        },
        serving,
    )
}

#[test]
fn a_session_404_reconnects_on_the_next_dispatch_without_repeating_the_call() {
    use kernel::Tool as _;

    let root = tempfile::tempdir().unwrap();
    let (server, serving) = session_ending_server();
    let residents = Residents::default();
    let (first, reached) = residents
        .tools(&server, root.path(), false, &no_secrets())
        .unwrap();
    assert!(matches!(reached, Reached::Connected(_)));
    let call = kernel::ToolCall {
        id: "first".to_owned(),
        name: first[0].meta().name.clone(),
        args: kernel::Payload::new(serde_json::Map::new()).unwrap(),
    };
    let refused = first[0].invoke(&call).unwrap_err();
    assert_eq!(
        (refused.code(), refused.retry()),
        (&kernel::AxCode::ToolUnavailable, kernel::Retry::Yes)
    );

    let (second, reached) = residents
        .tools(&server, root.path(), false, &no_secrets())
        .unwrap();
    assert!(
        matches!(reached, Reached::Connected(_)),
        "a tool's cloned link must invalidate the Residents connection"
    );
    assert_eq!(
        second[0].remote(),
        "pong",
        "the new handshake must take a fresh listing"
    );
    let unknown = second[0]
        .invoke(&kernel::ToolCall {
            name: second[0].meta().name.clone(),
            ..call
        })
        .unwrap_err();
    assert_eq!(unknown.retry(), kernel::Retry::Unknown);
    let (_, reached) = residents
        .tools(&server, root.path(), false, &no_secrets())
        .unwrap();
    assert!(
        matches!(reached, Reached::Resident),
        "an unread answer must not trigger another handshake"
    );
    let sent = serving.join().unwrap();
    for initialize in [0, 4] {
        assert!(
            !sent[initialize]
                .to_ascii_lowercase()
                .contains("mcp-session-id:")
        );
        assert!(
            !sent[initialize]
                .to_ascii_lowercase()
                .contains("mcp-protocol-version:")
        );
    }
    for (indexes, id, version) in [
        ([1, 2, 3], "old", "2025-03-26"),
        ([5, 6, 7], "new", agent_protocols::PROTOCOL_VERSION),
    ] {
        for index in indexes {
            let request = sent[index].to_ascii_lowercase();
            assert!(request.contains(&format!("mcp-session-id: {id}")));
            assert!(request.contains(&format!("mcp-protocol-version: {version}")));
        }
    }
    assert_eq!(
        sent.iter()
            .filter(|request| request.contains("\"method\":\"tools/call\""))
            .count(),
        2,
        "one original call and one new call; no automatic resend"
    );
}
