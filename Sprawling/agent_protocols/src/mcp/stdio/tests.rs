// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

    let second = server.call("{\"id\":2}", TimeoutMs(200)).unwrap_err();
    assert!(
        matches!(second.code(), &AxCode::ToolUnavailable | &AxCode::Timeout),
        "{second}"
    );
}

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

const PLANTED: &str = "SPRAWLING_SECRET_PROBE_KEY";

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
