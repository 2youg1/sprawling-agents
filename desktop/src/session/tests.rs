// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one connection is held to: the handshake order, the six names,
//! and a refusal for everything this build cannot carry out.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;

/// The two lines the city's own client sends first, verbatim from
/// `protocol::Rpc::initialize` and `protocol::Rpc::initialized`.
const OPENING: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":\
     {\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":\
     {\"name\":\"sprawling\",\"version\":\"0.0.5\"}}}";
const READY: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}";

fn opened(scope: &str) -> Server {
    let mut server = Server::new(Scope::parse(scope));
    server.answer(OPENING).expect("initialize is answered");
    assert!(server.answer(READY).is_none());
    server
}

fn answer(server: &mut Server, method: &str, params: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 9, "method": method, "params": params });
    let raw = server
        .answer(&line.to_string())
        .expect("a request with an id is answered");
    serde_json::from_str(&raw).expect("the answer is one JSON object")
}

/// The exchange the city opens every connection with, answered the way
/// `protocol::handshake` reads it.
#[test]
fn the_handshake_is_the_one_the_citys_client_already_speaks() {
    let mut server = Server::new(Scope::parse("windows = [\"*\"]\n"));
    let raw = server.answer(OPENING).expect("initialize is answered");
    let answer: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(answer["result"]["serverInfo"]["name"], "sprawling-desktop");
    assert!(answer["result"]["capabilities"]["tools"].is_object());
    assert!(
        server.answer(READY).is_none(),
        "a notification is never answered"
    );
    assert_eq!(server.phase, Phase::Ready);
}

/// Before the handshake finishes, nothing else is answered — which is
/// what the specification says and what keeps a client's ordering
/// mistake from surfacing somewhere else.
#[test]
fn work_asked_for_before_the_handshake_is_refused() {
    let mut server = Server::new(Scope::parse("windows = [\"*\"]\n"));
    for method in ["tools/list", "tools/call"] {
        let answer = answer(&mut server, method, json!({ "name": "desktop.windows" }));
        assert_eq!(answer["error"]["data"]["code"], "E_GATE_DENIED", "{method}");
        assert!(
            answer["error"]["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("initialize")
        );
    }
    // `ping` is answered at any time: it is how a caller learns the
    // process is alive.
    let alive = answer(&mut server, "ping", json!({}));
    assert!(alive["result"].is_object());
}

#[test]
fn the_list_carries_the_six_names_with_a_description_and_a_schema_each() {
    let mut server = opened("windows = [\"*\"]\n");
    let listed = answer(&mut server, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "desktop.windows",
            "desktop.snapshot",
            "desktop.act",
            "desktop.screenshot",
            "desktop.record",
            "desktop.clipboard",
        ]
    );
    for tool in tools {
        assert!(tool["description"].as_str().unwrap().contains("does not"));
        assert_eq!(tool["inputSchema"]["type"], "object");
    }
}

/// On a machine that is not this desktop, an admitted call still gets an
/// answer rather than a crash, and the answer names the platform.
#[cfg(not(windows))]
#[test]
fn elsewhere_an_admitted_call_is_refused_by_the_platform_rather_than_answered_falsely() {
    let mut server = opened("windows = [\"*Notepad*\"]\n");
    let answered = answer(
        &mut server,
        "tools/call",
        json!({ "name": "desktop.snapshot", "arguments": { "title": "a.txt — Notepad" } }),
    );
    assert_eq!(
        answered["error"]["data"]["code"], "E_TOOL_UNAVAILABLE",
        "{answered}"
    );
    assert!(answered["result"].is_null());
}

/// On Windows an admitted
/// `desktop.windows` reaches the desktop and answers with the windows
/// this scope lists — an array, possibly empty on a machine with nothing
/// open, and never `E_TOOL_UNAVAILABLE`.
#[cfg(windows)]
#[test]
fn listing_windows_reaches_this_desktop_rather_than_a_refusal() {
    let mut server = opened("windows = [\"*\"]\n");
    let answered = answer(
        &mut server,
        "tools/call",
        json!({ "name": "desktop.windows" }),
    );
    assert!(
        answered["error"].is_null(),
        "this build carries the desktop out: {answered}"
    );
    assert!(
        answered["result"]["windows"].is_array(),
        "the listing is an array: {answered}"
    );
}

/// A window nobody has open is a refusal that says how to find
/// out what is open — never a crash, and never an action landing on
/// whatever else was there.
#[cfg(windows)]
#[test]
fn a_window_this_machine_does_not_have_is_refused_with_the_way_to_find_out() {
    let mut server = opened("windows = [\"*\"]\n");
    let answered = answer(
        &mut server,
        "tools/call",
        json!({
            "name": "desktop.act",
            "arguments": {
                "title": "no window on this machine is called this — sprawling test",
                "action": "click",
                "generation": 0,
            },
        }),
    );
    assert_eq!(
        answered["error"]["data"]["code"], "E_INVALID_ARGS",
        "{answered}"
    );
    assert!(
        answered["error"]["data"]["recovery"]
            .as_str()
            .unwrap_or_default()
            .contains("desktop.windows"),
        "{answered}"
    );
}

/// The scope is judged before the platform is reached, so a window
/// nobody listed is refused with no call made.
#[test]
fn a_call_outside_the_scope_is_refused_before_the_platform_is_reached() {
    let mut server = opened("windows = [\"*Notepad*\"]\n");
    let answered = answer(
        &mut server,
        "tools/call",
        json!({ "name": "desktop.act", "arguments": { "title": "Password Manager" } }),
    );
    assert_eq!(answered["error"]["data"]["code"], "E_GATE_DENIED");
    assert!(
        answered["error"]["data"]["recovery"]
            .as_str()
            .unwrap()
            .contains("DESKTOP.toml")
    );
}

#[test]
fn a_method_and_a_tool_this_server_does_not_have_are_both_named_in_the_refusal() {
    let mut server = opened("windows = [\"*\"]\n");
    let method = answer(&mut server, "resources/list", json!({}));
    assert_eq!(method["error"]["code"], -32601);
    assert_eq!(method["error"]["data"]["code"], "E_TOOL_UNKNOWN");
    let tool = answer(
        &mut server,
        "tools/call",
        json!({ "name": "desktop.reboot", "arguments": {} }),
    );
    assert_eq!(tool["error"]["data"]["code"], "E_TOOL_UNKNOWN");
    let unnamed = answer(&mut server, "tools/call", json!({ "arguments": {} }));
    assert_eq!(unnamed["error"]["data"]["code"], "E_INVALID_ARGS");
}

/// The read loop answers one line per request, writes nothing for a
/// notification, and ends when the pipe closes.
#[test]
fn the_loop_writes_one_line_per_request_and_none_for_a_notification() {
    let mut server = Server::new(Scope::parse("windows = [\"*\"]\n"));
    let input =
        format!("{OPENING}\n{READY}\n{{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}}\n");
    let mut written: Vec<u8> = Vec::new();
    server
        .serve(std::io::Cursor::new(input), &mut written)
        .expect("the pipe closing is not a failure");
    let text = String::from_utf8(written).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    let ping: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(ping["id"], 3);
}
