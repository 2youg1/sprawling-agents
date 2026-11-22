// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use mei::acp::{self, Outcome, StopReason};
use serde_json::{Value, json};

fn read_line(reader: &mut impl BufRead) -> Value {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .expect("ACP stdout closed early");
    assert!(read > 0, "ACP stdout closed without a reply");
    let message: Value = serde_json::from_str(&line).expect("ACP reply must be JSON");
    if let Some(error) = message.get("error") {
        panic!("unexpected JSON-RPC error: {error}");
    }
    message
}

fn read_line_allow_error(reader: &mut impl BufRead) -> Value {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .expect("ACP stdout closed early");
    assert!(read > 0, "ACP stdout closed without a reply");
    serde_json::from_str(&line).expect("ACP reply must be JSON")
}

fn write_line(writer: &mut impl Write, message: Value) {
    writeln!(writer, "{}", serde_json::to_string(&message).unwrap()).unwrap();
    writer.flush().unwrap();
}

#[test]
fn prompt_text_keeps_nonempty_text_blocks() {
    assert_eq!(
        acp::prompt_text(&[
            json!({"type":"text","text":"a"}),
            json!({"type":"image"}),
            json!({"type":"text","text":"b"}),
        ]),
        Some("a\nb".to_owned())
    );
    assert_eq!(acp::prompt_text(&[json!({"type":"text","text":""})]), None);
    assert_eq!(
        acp::prompt_text(&[json!({
            "type":"resource_link",
            "name":"notes.txt",
            "uri":"file:///notes.txt"
        })]),
        Some("resource_link name=notes.txt uri=file:///notes.txt".to_owned())
    );
}

#[test]
fn parse_error_replies_minus_32700_and_keeps_serving() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().canonicalize().unwrap();
    let (agent_stdin, mut client_stdout) = std::io::pipe().unwrap();
    let (client_stdin, agent_stdout) = std::io::pipe().unwrap();
    let server = thread::spawn(move || {
        acp::serve(agent_stdin, agent_stdout, |_turn| {
            Ok(Outcome {
                stop: StopReason::EndTurn,
                text: "ok".to_owned(),
            })
        })
        .unwrap();
    });
    let mut reader = BufReader::new(client_stdin);
    writeln!(client_stdout, "{{").unwrap();
    client_stdout.flush().unwrap();
    let broken = read_line_allow_error(&mut reader);
    assert_eq!(broken["error"]["code"], -32_700);
    write_line(
        &mut client_stdout,
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1}}),
    );
    let init = read_line(&mut reader);
    assert_eq!(init["result"]["protocolVersion"], 1);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":1,"method":"session/new",
            "params":{"cwd": cwd.to_str().unwrap(), "mcpServers": []}
        }),
    );
    let _: Value = read_line(&mut reader);
    drop(client_stdout);
    server.join().unwrap();
}

#[test]
fn finish_prefers_cancel() {
    assert_eq!(
        acp::finish(true, StopReason::EndTurn),
        StopReason::Cancelled
    );
    assert_eq!(acp::finish(false, StopReason::EndTurn), StopReason::EndTurn);
}

#[test]
fn initialize_session_and_prompt_return_end_turn() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().canonicalize().unwrap();
    let (agent_stdin, mut client_stdout) = std::io::pipe().unwrap();
    let (client_stdin, agent_stdout) = std::io::pipe().unwrap();
    let server = thread::spawn(move || {
        acp::serve(agent_stdin, agent_stdout, |turn| {
            assert!(turn.first);
            assert_eq!(turn.prompt, "fix the kiln");
            Ok(Outcome {
                stop: StopReason::EndTurn,
                text: "done".to_owned(),
            })
        })
        .unwrap();
    });
    let mut reader = BufReader::new(client_stdin);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize",
            "params":{"protocolVersion":1,"clientInfo":{"name":"test","version":"0"}}
        }),
    );
    let init = read_line(&mut reader);
    assert_eq!(init["result"]["protocolVersion"], 1);
    assert_eq!(init["result"]["agentInfo"]["name"], "mei");

    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":1,"method":"session/new",
            "params":{"cwd": cwd.to_str().unwrap(), "mcpServers": []}
        }),
    );
    let opened = read_line(&mut reader);
    let session = opened["result"]["sessionId"].as_str().unwrap().to_owned();

    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":2,"method":"session/prompt",
            "params":{
                "sessionId": session,
                "prompt":[{"type":"text","text":"fix the kiln"}]
            }
        }),
    );
    let update = read_line(&mut reader);
    let answer = read_line(&mut reader);
    assert_eq!(update["method"], "session/update");
    assert_eq!(
        update["params"]["update"]["sessionUpdate"],
        "agent_message_chunk"
    );
    assert_eq!(update["params"]["update"]["content"]["text"], "done");
    assert_eq!(answer["result"]["stopReason"], "end_turn");
    drop(client_stdout);
    server.join().unwrap();
}

#[test]
fn cancel_after_a_turn_marks_cancelled() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().canonicalize().unwrap();
    let (agent_stdin, mut client_stdout) = std::io::pipe().unwrap();
    let (client_stdin, agent_stdout) = std::io::pipe().unwrap();
    let gate = Arc::new(AtomicBool::new(false));
    let wait = Arc::clone(&gate);
    let lines_read = Arc::new(AtomicU64::new(0));
    let meter = Arc::clone(&lines_read);
    let server = thread::spawn(move || {
        acp::serve_with_meter(
            agent_stdin,
            agent_stdout,
            move |_turn| {
                while !wait.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(Outcome {
                    stop: StopReason::EndTurn,
                    text: "late".to_owned(),
                })
            },
            Some(meter),
        )
        .unwrap();
    });
    let mut reader = BufReader::new(client_stdin);
    write_line(
        &mut client_stdout,
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1}}),
    );
    let _: Value = read_line(&mut reader);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":1,"method":"session/new",
            "params":{"cwd": cwd.to_str().unwrap(), "mcpServers": []}
        }),
    );
    let opened = read_line(&mut reader);
    let session = opened["result"]["sessionId"].as_str().unwrap().to_owned();
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":2,"method":"session/prompt",
            "params":{"sessionId": session, "prompt":[{"type":"text","text":"go"}]}
        }),
    );
    let before_cancel = wait_for_lines(&lines_read, 3);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","method":"session/cancel",
            "params":{"sessionId": session}
        }),
    );
    wait_for_lines(&lines_read, before_cancel.saturating_add(1));
    gate.store(true, Ordering::SeqCst);
    let mut stop = None;
    for _ in 0..8 {
        let message = read_line_allow_error(&mut reader);
        if let Some(error) = message.get("error") {
            panic!("prompt failed before cancel could finish: {error}");
        }
        if let Some(reason) = message
            .pointer("/result/stopReason")
            .and_then(Value::as_str)
        {
            stop = Some(reason.to_owned());
            break;
        }
    }
    assert_eq!(stop.as_deref(), Some("cancelled"));
    drop(client_stdout);
    server.join().unwrap();
}

#[test]
fn cancel_without_matching_session_id_is_ignored() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().canonicalize().unwrap();
    let (agent_stdin, mut client_stdout) = std::io::pipe().unwrap();
    let (client_stdin, agent_stdout) = std::io::pipe().unwrap();
    let gate = Arc::new(AtomicBool::new(false));
    let wait = Arc::clone(&gate);
    let lines_read = Arc::new(AtomicU64::new(0));
    let meter = Arc::clone(&lines_read);
    let server = thread::spawn(move || {
        acp::serve_with_meter(
            agent_stdin,
            agent_stdout,
            move |_turn| {
                while !wait.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(Outcome {
                    stop: StopReason::EndTurn,
                    text: "kept".to_owned(),
                })
            },
            Some(meter),
        )
        .unwrap();
    });
    let mut reader = BufReader::new(client_stdin);
    write_line(
        &mut client_stdout,
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1}}),
    );
    let _: Value = read_line(&mut reader);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":1,"method":"session/new",
            "params":{"cwd": cwd.to_str().unwrap(), "mcpServers": []}
        }),
    );
    let opened = read_line(&mut reader);
    let session = opened["result"]["sessionId"].as_str().unwrap().to_owned();
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":2,"method":"session/prompt",
            "params":{"sessionId": session, "prompt":[{"type":"text","text":"go"}]}
        }),
    );
    let before = wait_for_lines(&lines_read, 3);
    write_line(
        &mut client_stdout,
        json!({"jsonrpc":"2.0","method":"session/cancel","params":{}}),
    );
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","method":"session/cancel",
            "params":{"sessionId":42}
        }),
    );
    wait_for_lines(&lines_read, before.saturating_add(2));
    gate.store(true, Ordering::SeqCst);
    let mut stop = None;
    for _ in 0..8 {
        let message = read_line_allow_error(&mut reader);
        if let Some(reason) = message
            .pointer("/result/stopReason")
            .and_then(Value::as_str)
        {
            stop = Some(reason.to_owned());
            break;
        }
    }
    assert_eq!(stop.as_deref(), Some("end_turn"));
    drop(client_stdout);
    server.join().unwrap();
}

#[test]
fn prompt_without_cancel_keeps_end_turn() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().canonicalize().unwrap();
    let (agent_stdin, mut client_stdout) = std::io::pipe().unwrap();
    let (client_stdin, agent_stdout) = std::io::pipe().unwrap();
    let server = thread::spawn(move || {
        acp::serve(agent_stdin, agent_stdout, |_turn| {
            Ok(Outcome {
                stop: StopReason::EndTurn,
                text: "ok".to_owned(),
            })
        })
        .unwrap();
    });
    let mut reader = BufReader::new(client_stdin);
    write_line(
        &mut client_stdout,
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1}}),
    );
    let _: Value = read_line(&mut reader);
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":1,"method":"session/new",
            "params":{"cwd": cwd.to_str().unwrap(), "mcpServers": []}
        }),
    );
    let opened = read_line(&mut reader);
    let session = opened["result"]["sessionId"].as_str().unwrap().to_owned();
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","id":2,"method":"session/prompt",
            "params":{"sessionId": session, "prompt":[{"type":"text","text":"go"}]}
        }),
    );
    let update = read_line(&mut reader);
    let answer = read_line(&mut reader);
    assert_eq!(update["method"], "session/update");
    assert_eq!(answer["result"]["stopReason"], "end_turn");
    drop(client_stdout);
    server.join().unwrap();
}

fn wait_for_lines(meter: &AtomicU64, at_least: u64) -> u64 {
    for _ in 0..1_000 {
        let seen = meter.load(Ordering::SeqCst);
        if seen >= at_least {
            return seen;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!(
        "ACP reader did not enqueue {at_least} lines; saw {}",
        meter.load(Ordering::SeqCst)
    );
}

#[test]
#[expect(clippy::disallowed_methods, reason = "argument fixture process")]
fn cli_acp_rejects_prompt_flags() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["--acp", "-p", "no"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_USAGE"), "{error}");
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "fixture locates the host python executable and restores the process cwd"
)]
fn python_tool_cwd_matches_session_cwd_not_spawn_cwd() {
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Err(err) = std::env::set_current_dir(&self.0) {
                eprintln!("restore process cwd after ACP cwd fixture: {err}");
            }
        }
    }

    let spawn_home = tempfile::tempdir().unwrap();
    let session_home = tempfile::tempdir().unwrap();
    let session = session_home.path().canonicalize().unwrap();
    std::fs::write(session.join("marker.txt"), "session-tree").unwrap();
    std::fs::write(spawn_home.path().join("marker.txt"), "spawn-tree").unwrap();
    let _restore = Restore(std::env::current_dir().unwrap());
    std::env::set_current_dir(spawn_home.path()).unwrap();

    let executable = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
        .args(["-c", "import sys; print(sys.executable, end='')"])
        .output()
        .unwrap();
    assert!(executable.status.success());
    let executable = std::path::PathBuf::from(String::from_utf8(executable.stdout).unwrap());
    let environment = mei::doctor::Environment::new(
        mei::doctor::Program::new("fixture", executable.clone()),
        mei::doctor::Program::new("fixture", executable),
    );
    let mut python = None;
    let mut bound = None;
    mei::launch::bind_python_to_cwd(&mut python, &mut bound, &session, &environment).unwrap();
    assert_eq!(bound.as_ref(), Some(&session));
    let python = python.as_mut().unwrap();
    let cwd = python.run("import os\nprint(os.getcwd(), end='')").unwrap();
    assert_eq!(
        std::path::Path::new(cwd.as_str().unwrap())
            .canonicalize()
            .unwrap(),
        session
    );
    let marker = python
        .run("print(open('marker.txt', encoding='utf-8').read(), end='')")
        .unwrap();
    assert_eq!(marker.as_str().unwrap(), "session-tree");
    python.close().unwrap();
}
