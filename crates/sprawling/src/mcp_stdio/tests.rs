// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the child-process transport is held to: a real child answering
//! over a real pipe, a deadline that ends the process rather than
//! risking two answers swapped, and one process behind every handle.

use super::*;
use protocol::Outbound;

fn silent_server() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                "Start-Sleep -Seconds 30".to_owned(),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), "sleep 30".to_owned()],
        )
    }
}

#[test]
fn a_real_child_answers_over_the_pipe_and_the_answer_reads_as_a_result() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = echoing("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}");
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let mut rpc = protocol::Rpc::new();
    let line = rpc.list_tools();
    let answer = server
        .call(&line, protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap();
    let result = protocol::Rpc::read(&answer).unwrap();
    assert!(result.get("tools").is_some(), "{result}");
}

#[test]
fn a_request_carrying_a_newline_is_refused_before_it_becomes_two_messages() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = echoing("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}");
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let err = server
        .call("{\"a\":\n\"b\"}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::WireMismatch);
    assert!(err.subject().contains("newline"));
}

#[test]
fn a_server_that_never_answers_is_given_up_on_and_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = silent_server();
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let err = server.call("{\"id\":1}", TimeoutMs(200)).unwrap_err();
    assert_eq!(err.code(), &AxCode::Timeout);
    assert!(err.recovery().contains("late answer"));
    // The child was stopped, so the next call cannot be answered by
    // the one that never arrived.
    let second = server.call("{\"id\":2}", TimeoutMs(200)).unwrap_err();
    assert!(
        matches!(second.code(), &AxCode::ToolUnavailable | &AxCode::Timeout),
        "{second}"
    );
}

#[test]
fn a_program_this_machine_cannot_start_refuses_with_the_command_in_it() {
    let dir = tempfile::tempdir().unwrap();
    let err = StdioServer::start("sprawling-no-such-server", &[], &[], dir.path()).unwrap_err();
    assert_eq!(err.code(), &AxCode::ToolUnavailable);
    assert!(err.subject().contains("sprawling-no-such-server"));
    assert!(err.recovery().contains("[[mcp]]"));
}

#[test]
fn two_handles_are_two_tools_talking_to_one_process() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = echoing("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":1}}");
    let server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let mut first = server.clone();
    let mut second = server.clone();
    assert!(
        first
            .call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
            .is_ok()
    );
    assert!(
        second
            .call("{\"id\":2}", protocol::EXTERNAL_CALL_PATIENCE)
            .is_ok()
    );
}

/// A name handed to a child cannot be taken back, so what a
/// building writes beside a server is exactly what that server is
/// started with - and the child is the only thing that can prove it.
#[test]
fn what_a_building_writes_beside_a_server_reaches_the_child() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = reporting_its_environment();
    let refuse_every_reference: gateway::SecretResolver =
        Box::new(|reference: &kernel::SecretRef| {
            Err(AxError::failure(
                AxCode::CredentialMissing,
                "resolve a credential",
                reference.to_string(),
            )
            .with_recovery("store it first"))
        });
    let env = crate::mcp_redeeming::redeem(
        &[("SPRAWLING_TEST_KEY".to_owned(), "opaque-value".to_owned())],
        &refuse_every_reference,
        "start an mcp server",
    )
    .unwrap();
    let mut server = StdioServer::start(&command, &args, &env, dir.path()).unwrap();
    let answer = server
        .call("{\"id\":1}", protocol::EXTERNAL_CALL_PATIENCE)
        .unwrap();
    let result = protocol::Rpc::read(&answer).unwrap();
    assert_eq!(
        result.get("seen").and_then(serde_json::Value::as_str),
        Some("opaque-value")
    );
}

/// A child that answers every message with the one variable it was
/// started with.
fn reporting_its_environment() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                "while($l=[Console]::In.ReadLine()){Write-Output \
                     ('{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"seen\":\"'\
                     +$env:SPRAWLING_TEST_KEY+'\"}}')}"
                    .to_owned(),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                "while IFS= read -r l; do printf \
                     '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"seen\":\"%s\"}}\n' \
                     \"$SPRAWLING_TEST_KEY\"; done"
                    .to_owned(),
            ],
        )
    }
}
