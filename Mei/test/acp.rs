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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mei::acp::{self, Outcome, StopReason};
use serde_json::{Value, json};

fn read_line(reader: &mut impl BufRead) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
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
}

#[test]
fn finish_prefers_cancel() {
    assert_eq!(
        acp::finish(true, StopReason::EndTurn),
        StopReason::Cancelled
    );
    assert_eq!(
        acp::finish(false, StopReason::EndTurn),
        StopReason::EndTurn
    );
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
    let server = thread::spawn(move || {
        acp::serve(agent_stdin, agent_stdout, move |_turn| {
            while !wait.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(Outcome {
                stop: StopReason::EndTurn,
                text: "late".to_owned(),
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
    thread::sleep(Duration::from_millis(30));
    write_line(
        &mut client_stdout,
        json!({
            "jsonrpc":"2.0","method":"session/cancel",
            "params":{"sessionId": session}
        }),
    );
    gate.store(true, Ordering::SeqCst);
    let mut stop = None;
    for _ in 0..8 {
        let message = read_line(&mut reader);
        if let Some(reason) = message.pointer("/result/stopReason").and_then(Value::as_str) {
            stop = Some(reason.to_owned());
            break;
        }
    }
    assert_eq!(stop.as_deref(), Some("cancelled"));
    drop(client_stdout);
    server.join().unwrap();
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
