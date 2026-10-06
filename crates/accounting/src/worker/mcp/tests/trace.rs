// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HttpFailure {
    BeforeHandover,
    AfterHandover,
    NotFound,
}

struct TraceServer {
    declaration: kernel::McpServer,
    stop: std::sync::mpsc::Sender<()>,
    serving: Option<std::thread::JoinHandle<Vec<(String, serde_json::Value)>>>,
}

impl TraceServer {
    fn start(failures: Vec<HttpFailure>, session: bool) -> Self {
        use std::io::Write as _;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let (stop, stopped) = std::sync::mpsc::channel();
        let serving = std::thread::spawn(move || {
            let mut seen = Vec::new();
            let mut generation = 0usize;
            loop {
                let stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        match stopped.recv_timeout(std::time::Duration::from_millis(5)) {
                            Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                break;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        }
                    }
                    Err(error) => panic!("accept a trace request: {error}"),
                };
                let (mut reader, headers, request) = read_http_request(stream);
                let (status, handed, body) = match request["method"].as_str().unwrap() {
                    "initialize" => {
                        generation = generation.saturating_add(1);
                        (
                            200,
                            if session {
                                format!("mcp-session-id: session-{generation}\r\n")
                            } else {
                                String::new()
                            },
                            serde_json::to_vec(&serde_json::json!({
                                "jsonrpc": "2.0", "id": request["id"], "result": {
                                    "protocolVersion": agent_protocols::PROTOCOL_VERSION,
                                    "serverInfo": {"name": "trace"}
                                }
                            }))
                            .unwrap(),
                        )
                    }
                    "notifications/initialized" => (202, String::new(), Vec::new()),
                    "tools/list" => (
                        200,
                        String::new(),
                        serde_json::to_vec(&serde_json::json!({
                            "jsonrpc": "2.0", "id": request["id"], "result": {
                                "tools": [{"name": format!("tool{generation}")}]
                            }
                        }))
                        .unwrap(),
                    ),
                    "tools/call" => {
                        let step = usize::try_from(
                            request["params"]["arguments"]["step"].as_u64().unwrap(),
                        )
                        .unwrap();
                        match failures[step] {
                            HttpFailure::BeforeHandover => {
                                seen.push((headers, request));
                                drop(reader);
                                continue;
                            }
                            HttpFailure::AfterHandover => (200, String::new(), vec![0xff]),
                            HttpFailure::NotFound => (404, String::new(), Vec::new()),
                        }
                    }
                    other => panic!("unexpected trace method: {other}"),
                };
                seen.push((headers, request));
                let response = format!(
                    "HTTP/1.1 {status} X\r\n{handed}content-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len()
                );
                reader.get_mut().write_all(response.as_bytes()).unwrap();
                reader.get_mut().write_all(&body).unwrap();
            }
            seen
        });
        Self {
            declaration: kernel::McpServer {
                label: kernel::ServerLabel::parse("trace").unwrap(),
                transport: kernel::McpTransport::Http {
                    url,
                    headers: vec![("X-Api-Key".to_owned(), "secret:trace/key".to_owned())],
                },
            },
            stop,
            serving: Some(serving),
        }
    }

    fn finish(mut self) -> Vec<(String, serde_json::Value)> {
        self.stop.send(()).unwrap();
        self.serving.take().unwrap().join().unwrap()
    }
}

impl Drop for TraceServer {
    fn drop(&mut self) {
        if let Some(serving) = self.serving.take() {
            // A shrinking assertion failure must still reclaim the fixture thread.
            match self.stop.send(()) {
                Ok(()) | Err(_) => {}
            }
            if std::thread::panicking() {
                drop(serving.join());
            } else {
                serving.join().unwrap();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitialLifetime {
    LiveSession,
    LiveStateless,
    Ended,
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    })]

    /// Derived from Link.lean's ended_remains_ended,
    /// session_ended_requires_connection and other_failures_preserve_lifetime.
    #[test]
    fn failure_traces_keep_shared_lifetime_and_choose_the_next_dispatch(
        initial in proptest::prop_oneof![
            proptest::strategy::Just(InitialLifetime::LiveSession),
            proptest::strategy::Just(InitialLifetime::LiveStateless),
            proptest::strategy::Just(InitialLifetime::Ended),
        ],
        trace in proptest::prop_oneof![
            proptest::collection::vec(
                (proptest::prop_oneof![
                    proptest::strategy::Just(HttpFailure::BeforeHandover),
                    proptest::strategy::Just(HttpFailure::AfterHandover),
                    proptest::strategy::Just(HttpFailure::NotFound),
                ], proptest::bool::ANY), 0..32
            ),
            proptest::collection::vec(
                (proptest::prop_oneof![
                    proptest::strategy::Just(HttpFailure::BeforeHandover),
                    proptest::strategy::Just(HttpFailure::AfterHandover),
                ], proptest::bool::ANY), 0..32
            ),
        ],
    ) {
        drive_failure_trace(initial, &trace);
    }
}

#[test]
fn an_ended_session_with_an_empty_trace_reconnects() {
    drive_failure_trace(InitialLifetime::Ended, &[]);
}

#[test]
fn a_live_session_with_not_found_invalidates_both_handles() {
    for cloned in [false, true] {
        drive_failure_trace(
            InitialLifetime::LiveSession,
            &[(HttpFailure::NotFound, cloned)],
        );
    }
}

fn drive_failure_trace(initial: InitialLifetime, trace: &[(HttpFailure, bool)]) {
    use kernel::Tool as _;
    let root = tempfile::tempdir().unwrap();
    let initially_ended = initial == InitialLifetime::Ended;
    let session = initial != InitialLifetime::LiveStateless;
    let failures: Vec<_> = std::iter::once(HttpFailure::NotFound)
        .take(usize::from(initially_ended))
        .chain(trace.iter().map(|(failure, _)| *failure))
        .collect();
    let fixture = TraceServer::start(failures.clone(), session);
    let resolve: gateway::SecretResolver =
        Box::new(|_| Ok(kernel::Sealed::new(Box::new("fixture-key".to_owned()))));
    let residents = Residents::default();
    let (first, reached) = residents
        .tools(&fixture.declaration, root.path(), false, &resolve)
        .unwrap();
    assert!(matches!(reached, Reached::Connected(_)));
    let (cloned, reached) = residents
        .tools(&fixture.declaration, root.path(), false, &resolve)
        .unwrap();
    assert!(matches!(reached, Reached::Resident));
    let ending = failures
        .iter()
        .position(|failure| session && *failure == HttpFailure::NotFound);
    for (step, failure) in failures.iter().enumerate() {
        let tool = if trace
            .get(step.saturating_sub(usize::from(initially_ended)))
            .is_some_and(|(_, clone)| *clone)
        {
            &cloned[0]
        } else {
            &first[0]
        };
        let refused = tool
            .invoke(&kernel::ToolCall {
                id: format!("call-{step}"),
                name: tool.meta().name.clone(),
                args: kernel::Payload::new(serde_json::Map::from_iter([(
                    "step".to_owned(),
                    serde_json::json!(step),
                )]))
                .unwrap(),
            })
            .unwrap_err();
        let expected = if ending.is_some_and(|ended| step > ended) {
            (&kernel::AxCode::ToolUnavailable, kernel::Retry::No)
        } else {
            match failure {
                HttpFailure::BeforeHandover => {
                    (&kernel::AxCode::ToolUnavailable, kernel::Retry::No)
                }
                HttpFailure::AfterHandover => {
                    (&kernel::AxCode::WireMismatch, kernel::Retry::Unknown)
                }
                HttpFailure::NotFound => (
                    &kernel::AxCode::ToolUnavailable,
                    if session {
                        kernel::Retry::Yes
                    } else {
                        kernel::Retry::No
                    },
                ),
            }
        };
        assert_eq!(
            (refused.code(), refused.retry()),
            expected,
            "step {step}: {failures:?}"
        );
    }
    if let Some(step) = ending {
        for tool in [&first[0], &cloned[0]] {
            let refused = tool
                .invoke(&kernel::ToolCall {
                    id: "old-handle".to_owned(),
                    name: tool.meta().name.clone(),
                    args: kernel::Payload::new(serde_json::Map::from_iter([(
                        "step".to_owned(),
                        serde_json::json!(step),
                    )]))
                    .unwrap(),
                })
                .unwrap_err();
            assert_eq!(
                (refused.code(), refused.retry()),
                (&kernel::AxCode::ToolUnavailable, kernel::Retry::No)
            );
        }
    }
    let (next, reached) = residents
        .tools(&fixture.declaration, root.path(), false, &resolve)
        .unwrap();
    assert_eq!(matches!(reached, Reached::Connected(_)), ending.is_some());
    assert_eq!(
        next[0].remote(),
        if ending.is_some() { "tool2" } else { "tool1" }
    );
    let seen = fixture.finish();
    let sent_calls = ending.map_or(failures.len(), |step| step.saturating_add(1));
    let opening = ["initialize", "notifications/initialized", "tools/list"];
    let expected: Vec<_> = opening
        .into_iter()
        .chain(std::iter::repeat_n("tools/call", sent_calls))
        .chain(
            opening
                .into_iter()
                .take(if ending.is_some() { 3 } else { 0 }),
        )
        .collect();
    assert_eq!(
        seen.iter()
            .map(|(_, body)| body["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected
    );
    let mut generation = 0usize;
    for (headers, request) in &seen {
        let headers = headers.to_ascii_lowercase();
        assert!(headers.contains("x-api-key: fixture-key"));
        assert!(!headers.contains("secret:"));
        if request["method"] == "initialize" {
            generation = generation.saturating_add(1);
            assert!(!headers.contains("mcp-session-id:"));
            assert!(!headers.contains("mcp-protocol-version:"));
        } else {
            assert_eq!(headers.contains("mcp-session-id:"), session);
            if session {
                assert!(headers.contains(&format!("mcp-session-id: session-{generation}\r\n")));
            }
            assert!(headers.contains(&format!(
                "mcp-protocol-version: {}",
                agent_protocols::PROTOCOL_VERSION
            )));
        }
    }
    assert_eq!(
        seen.iter()
            .filter(|(_, body)| body["method"] == "tools/call")
            .map(|(_, body)| body["params"]["arguments"]["step"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        (0..sent_calls)
            .map(|step| u64::try_from(step).unwrap())
            .collect::<Vec<_>>()
    );
}
