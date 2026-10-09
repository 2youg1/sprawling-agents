// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the child-process transport is held to: a real child answering
//! over a real pipe, a deadline that ends the process rather than
//! risking two answers swapped, and one process behind every handle.

use super::*;
use crate::Outbound;

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

/// A child that reads the one line it is sent and then exits without an
/// answer: the call was handed over whole, so what it did is unknown.
fn taking_one_line_then_exiting() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                "[void][Console]::In.ReadLine()".to_owned(),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec!["-c".to_owned(), "read -r line".to_owned()],
        )
    }
}

#[test]
fn a_real_child_answers_over_the_pipe_and_the_answer_reads_as_a_result() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = echoing("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}");
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let mut rpc = crate::Rpc::new();
    let line = rpc.list_tools();
    let answer = server.call(&line, crate::EXTERNAL_CALL_PATIENCE).unwrap();
    let result = crate::Rpc::read(&answer).unwrap();
    assert!(result.get("tools").is_some(), "{result}");
}

#[test]
fn a_request_carrying_a_newline_is_refused_before_it_becomes_two_messages() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = echoing("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}");
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let err = server
        .call("{\"a\":\n\"b\"}", crate::EXTERNAL_CALL_PATIENCE)
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
    assert_eq!(
        (err.code(), err.retry()),
        (&AxCode::Timeout, kernel::Retry::Unknown)
    );
    assert!(err.recovery().contains("late answer"));
    // The child was stopped, so the next call cannot be answered by
    // the one that never arrived.
    let second = server.call("{\"id\":2}", TimeoutMs(200)).unwrap_err();
    assert!(
        matches!(second.code(), &AxCode::ToolUnavailable | &AxCode::Timeout),
        "{second}"
    );
}

/// The line was written and flushed before the child went away, so the
/// server may have acted on it: the lost answer is not a call to repeat.
#[test]
fn a_server_that_exits_after_taking_the_call_leaves_the_effect_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = taking_one_line_then_exiting();
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let err = server
        .call("{\"id\":1}", crate::EXTERNAL_CALL_PATIENCE)
        .unwrap_err();
    assert_eq!(
        (err.code(), err.retry()),
        (&AxCode::ToolUnavailable, kernel::Retry::Unknown)
    );
}

/// A child that reads the one line it is sent and answers with one line
/// a byte longer than the ceiling.
fn answering_past_the_ceiling() -> (String, Vec<String>) {
    let over = crate::MESSAGE_CEILING + 1;
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                format!(
                    "[void][Console]::In.ReadLine(); [Console]::Out.Write('x' * {over}); \
                     [Console]::Out.WriteLine()"
                ),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                format!("read -r line; head -c {over} /dev/zero | tr '\\0' x; echo"),
            ],
        )
    }
}

/// The reader stops at the ceiling rather than growing one line without
/// bound, and the call that was waiting learns why. The server did
/// answer, so what it did is unknown; the child is stopped, because the
/// rest of that line cannot be told apart from the next message.
#[test]
fn an_answer_past_the_ceiling_is_refused_and_the_server_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = answering_past_the_ceiling();
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let answered = server.call("{\"id\":1}", crate::EXTERNAL_CALL_PATIENCE);
    assert_eq!(
        answered
            .map(|answer| answer.len())
            .map_err(|err| (*err.code(), err.retry())),
        Err((AxCode::WireMismatch, kernel::Retry::Unknown))
    );
    assert!(server.has_ended());
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
            .call("{\"id\":1}", crate::EXTERNAL_CALL_PATIENCE)
            .is_ok()
    );
    assert!(
        second
            .call("{\"id\":2}", crate::EXTERNAL_CALL_PATIENCE)
            .is_ok()
    );
}

/// A name handed to a child cannot be taken back, so what a
/// building writes beside a server is exactly what that server is
/// started with - and the child is the only thing that can prove it.
#[test]
fn what_a_building_writes_beside_a_server_reaches_the_child() {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = reporting_its_environment("SPRAWLING_TEST_KEY");
    let refuse_every_reference: gateway::SecretResolver =
        Box::new(|reference: &kernel::SecretRef| {
            Err(AxError::failure(
                AxCode::CredentialMissing,
                "resolve a credential",
                reference.to_string(),
            )
            .with_recovery("store it first"))
        });
    let env = crate::mcp::redeeming::redeem(
        &[("SPRAWLING_TEST_KEY".to_owned(), "opaque-value".to_owned())],
        &refuse_every_reference,
        "start an mcp server",
    )
    .unwrap();
    let mut server = StdioServer::start(&command, &args, &env, dir.path()).unwrap();
    let answer = server
        .call("{\"id\":1}", crate::EXTERNAL_CALL_PATIENCE)
        .unwrap();
    let result = crate::Rpc::read(&answer).unwrap();
    assert_eq!(
        result.get("seen").and_then(serde_json::Value::as_str),
        Some("opaque-value")
    );
}

/// A child that answers every message with the value of the variable
/// `name` in its own environment, empty when it has none.
fn reporting_its_environment(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_owned(),
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                format!(
                    "while($l=[Console]::In.ReadLine()){{Write-Output \
                     ('{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"seen\":\"'\
                     +$env:{name}+'\"}}}}')}}"
                ),
            ],
        )
    } else {
        (
            "sh".to_owned(),
            vec![
                "-c".to_owned(),
                format!(
                    "while IFS= read -r l; do printf \
                     '{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"seen\":\"%s\"}}}}\n' \
                     \"${name}\"; done"
                ),
            ],
        )
    }
}

/// The name of the secret the test below plants: a key under the vault's
/// prefix, so it is one of the city's secrets by the rule every child
/// process is started under.
const PLANTED: &str = "SPRAWLING_SECRET_PROBE_KEY";

/// Found in the security scan: an MCP server inherited the city's whole
/// environment, so every `SPRAWLING_SECRET_*` key the vault reads reached
/// whatever package the server's command pulled in. A test cannot set a
/// variable in its own process without `unsafe`, so it starts its own
/// binary again with the key set, and that copy starts the server.
#[test]
fn a_server_inherits_none_of_the_city_secret_keys() {
    match std::env::var(PLANTED) {
        Ok(planted) => {
            assert_eq!(
                planted, "planted",
                "the copy runs with the key it was given"
            );
            assert_eq!(seen_by_a_server(PLANTED), "");
        }
        Err(_) => {
            let ran = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "mcp::stdio::tests::a_server_inherits_none_of_the_city_secret_keys",
                    "--nocapture",
                ])
                .env(PLANTED, "planted")
                .output()
                .unwrap();
            let said = String::from_utf8_lossy(&ran.stdout);
            assert!(
                ran.status.success(),
                "{said}{}",
                String::from_utf8_lossy(&ran.stderr)
            );
            assert!(said.contains("1 passed"), "the copy ran no test: {said}");
        }
    }
}

/// What a server started by this process finds in `name`.
fn seen_by_a_server(name: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let (command, args) = reporting_its_environment(name);
    let mut server = StdioServer::start(&command, &args, &[], dir.path()).unwrap();
    let answer = server
        .call("{\"id\":1}", crate::EXTERNAL_CALL_PATIENCE)
        .unwrap();
    crate::Rpc::read(&answer)
        .unwrap()
        .get("seen")
        .and_then(serde_json::Value::as_str)
        .unwrap()
        .to_owned()
}
