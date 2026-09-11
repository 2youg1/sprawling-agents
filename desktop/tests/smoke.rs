// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one test that starts the real binary.
//!
//! Everything else in this package proves something about a library
//! function. This proves the claim the card is actually about: a city
//! that puts a `command` entry in a building's `CONFIG.toml` gets a
//! working MCP server, with no special casing anywhere in the assembly
//! layer. The exchange below is the one `protocol::handshake` and
//! `protocol::Rpc::list_tools` send, byte for byte.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn the_binary_answers_an_initialize_and_a_tools_list_over_its_own_pipes() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sprawling-desktop"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server binary starts");
    {
        let requests = child.stdin.as_mut().expect("the child was given pipes");
        for line in [
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":\
             {\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":\
             {\"name\":\"sprawling\",\"version\":\"0.0.4\"}}}",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}",
        ] {
            writeln!(requests, "{line}").expect("the request reaches the child");
        }
        requests.flush().expect("the request is not left buffered");
    }
    // Closing stdin ends the child's read loop, so the process finishes
    // on its own and this test cannot hang on a pipe nobody closed.
    let stdout = child.stdout.take().expect("the child was given pipes");
    let mut answers = BufReader::new(stdout).lines();
    drop(child.stdin.take());

    let opened: serde_json::Value =
        serde_json::from_str(&answers.next().expect("initialize is answered").unwrap()).unwrap();
    assert_eq!(opened["id"], 1);
    assert_eq!(opened["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(opened["result"]["serverInfo"]["name"], "sprawling-desktop");

    let listed: serde_json::Value =
        serde_json::from_str(&answers.next().expect("tools/list is answered").unwrap()).unwrap();
    assert_eq!(listed["id"], 2);
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("the list is an array")
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
    assert!(
        answers.next().is_none(),
        "a notification was answered, which would put the pipe one line out of step"
    );
    let ended = child.wait().expect("the child ends when its input closes");
    assert!(ended.success(), "the server ended with {ended}");
}
