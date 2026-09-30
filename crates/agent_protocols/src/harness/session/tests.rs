// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An agent played in memory on the far side of two pipes: it reads
//! what the session sends, line by line, and answers from a script.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};

use super::*;

/// What the played agent does once the session has sent it a request.
enum Step {
    /// Answer the request just read with this result.
    Answer(Value),
    /// Send this message unprompted.
    Say(Value),
    /// Send this request and read the client's answer to it.
    Ask(Value),
    /// Read one more request from the client.
    Read,
    /// Close the output without another word.
    Hang,
    /// Say nothing for this long.
    Wait(Duration),
    /// Read everything the session still sends, until it closes its end.
    Drain,
}

/// Starts an agent that follows `script`, and hands back the session's
/// two ends and what the agent read.
fn played(
    script: Vec<Step>,
) -> (
    BufReader<std::io::PipeReader>,
    std::io::PipeWriter,
    JoinHandle<Vec<Value>>,
) {
    let (from_agent, mut agent_out) = std::io::pipe().unwrap();
    let (agent_in, to_agent) = std::io::pipe().unwrap();
    let agent = std::thread::spawn(move || {
        let mut heard = Vec::new();
        let mut lines = BufReader::new(agent_in);
        let mut read = |heard: &mut Vec<Value>| {
            let mut line = String::new();
            if lines.read_line(&mut line).unwrap() == 0 {
                return None;
            }
            let message: Value = serde_json::from_str(&line).unwrap();
            heard.push(message.clone());
            Some(message)
        };
        let mut last = read(&mut heard).unwrap();
        for step in script {
            let said = match step {
                Step::Answer(result) => {
                    json!({ "jsonrpc": "2.0", "id": last["id"], "result": result })
                }
                Step::Say(message) | Step::Ask(message) => message.clone(),
                Step::Read => {
                    last = read(&mut heard).unwrap();
                    continue;
                }
                Step::Hang => return heard,
                Step::Wait(quiet) => {
                    std::thread::sleep(quiet);
                    continue;
                }
                Step::Drain => {
                    while read(&mut heard).is_some() {}
                    continue;
                }
            };
            writeln!(agent_out, "{said}").unwrap();
            agent_out.flush().unwrap();
            if said.get("method").is_some() && said.get("id").is_some() {
                read(&mut heard);
            }
        }
        heard
    });
    (BufReader::new(from_agent), to_agent, agent)
}

/// The session's end of a played agent, read the way production reads a
/// child's output.
fn opened(
    script: Vec<Step>,
    name: &str,
) -> (AcpSession<std::io::PipeWriter>, JoinHandle<Vec<Value>>) {
    let (reader, writer, agent) = played(script);
    let lines = Lines::over(reader, name).unwrap();
    let session = AcpSession::open(lines, writer, name, Path::new("/room")).unwrap();
    (session, agent)
}

fn opening() -> Vec<Step> {
    vec![
        Step::Answer(json!({ "protocolVersion": 1, "agentCapabilities": {} })),
        Step::Read,
        Step::Answer(json!({ "sessionId": "s1" })),
        Step::Read,
    ]
}

/// A whole turn: the agent reports, asks, reaches for a capability this
/// client never offered, and ends. Every report arrives in order, the
/// ask is answered with the caller's choice, and the reach is refused
/// by name rather than left hanging.
#[test]
fn a_turn_hands_back_what_the_agent_reported_and_answers_what_it_asked() {
    let mut script = opening();
    script.extend([
        Step::Say(json!({ "jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s1",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "reading" } }
        }})),
        Step::Say(json!({ "jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s1",
            "update": { "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read Cargo.toml", "kind": "read" }
        }})),
        Step::Ask(json!({ "jsonrpc": "2.0", "id": 90, "method": "session/request_permission", "params": {
            "sessionId": "s1",
            "toolCall": { "toolCallId": "t2", "title": "Run cargo test" },
            "options": [
                { "optionId": "yes", "name": "Allow", "kind": "allow_once" },
                { "optionId": "no", "name": "Reject", "kind": "reject_once" }
            ]
        }})),
        Step::Ask(json!({ "jsonrpc": "2.0", "id": 91, "method": "fs/read_text_file", "params": {
            "sessionId": "s1", "path": "/etc/hosts"
        }})),
        Step::Answer(json!({ "stopReason": "end_turn" })),
    ]);
    let (mut session, agent) = opened(script, "grok_build");
    let mut heard = Vec::new();
    let mut asked = Vec::new();
    let answer = session
        .prompt(
            "fix the build",
            &mut Listener {
                halted: &mut || false,
                cancelling: &mut || Ok(()),
                report: &mut |update| {
                    heard.push(update);
                    Ok(())
                },
                permit: &mut |ask| {
                    asked.push(ask.clone());
                    Permit::Chosen("no".to_owned())
                },
            },
        )
        .unwrap();
    assert_eq!(
        answer,
        Answer {
            stop: StopReason::EndTurn,
            text: "reading".to_owned(),
        }
    );
    assert_eq!(
        heard,
        vec![
            Update::Text("reading".to_owned()),
            Update::ToolCall {
                id: "t1".to_owned(),
                title: "Read Cargo.toml".to_owned(),
                kind: "read".to_owned(),
            },
        ]
    );
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].title, "Run cargo test");
    assert_eq!(asked[0].options[1].kind, PermitKind::RejectOnce);
    let wire = agent.join().unwrap();
    assert_eq!(
        wire[0]["params"]["clientCapabilities"],
        json!({ "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false })
    );
    assert_eq!(
        wire[1]["params"],
        json!({ "cwd": "/room", "mcpServers": [] })
    );
    assert_eq!(wire[2]["params"]["sessionId"], "s1");
    assert_eq!(
        wire[3]["result"],
        json!({ "outcome": { "outcome": "selected", "optionId": "no" } })
    );
    assert_eq!(wire[4]["error"]["code"], -32_601);
}

/// An agent that stops talking in the middle of a turn may already
/// have acted, so the refusal says the effect is unknown rather than
/// inviting a blind retry.
#[test]
fn an_agent_that_goes_quiet_mid_turn_leaves_the_effect_unknown() {
    let mut script = opening();
    script.push(Step::Hang);
    let (mut session, agent) = opened(script, "pi");
    let refused = session.prompt("go", &mut quiet()).unwrap_err();
    assert_eq!(*refused.code(), AxCode::Provider);
    assert_eq!(refused.retry(), kernel::Retry::Unknown);
    agent.join().unwrap();
}

/// A stop reason this client cannot name is refused, not guessed.
#[test]
fn a_stop_reason_nobody_defined_is_refused() {
    let mut script = opening();
    script.push(Step::Answer(json!({ "stopReason": "tired" })));
    let (mut session, agent) = opened(script, "codex");
    let refused = session.prompt("go", &mut quiet()).unwrap_err();
    assert_eq!(*refused.code(), AxCode::WireMismatch);
    agent.join().unwrap();
}

/// A listener nothing happens to: no halt, and every report and ask let
/// through unrecorded.
fn quiet() -> Listener<'static> {
    Listener {
        halted: Box::leak(Box::new(|| false)),
        cancelling: Box::leak(Box::new(|| Ok(()))),
        report: Box::leak(Box::new(|_| Ok(()))),
        permit: Box::leak(Box::new(|_| Permit::Cancelled)),
    }
}

/// A halt that arrives while the agent says nothing still becomes a
/// cancel, once, and the cancel is booked and sent before the next
/// report is handed on (crates/agent_protocols/spec/Harness/Session.lean,
/// `a_halt_is_a_cancel_before_anything_else`).
#[test]
fn a_halt_while_the_agent_is_silent_is_a_cancel_before_the_next_report() {
    let mut script = opening();
    script.extend([
        Step::Wait(Duration::from_millis(600)),
        Step::Say(json!({ "jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": "s1",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "late" } }
        }})),
        Step::Answer(json!({ "stopReason": "cancelled" })),
        Step::Drain,
    ]);
    let (mut session, agent) = opened(script, "kimi_code");
    let order = RefCell::new(Vec::new());
    let answer = session
        .prompt(
            "go",
            &mut Listener {
                halted: &mut || true,
                cancelling: &mut || {
                    order.borrow_mut().push("cancel booked".to_owned());
                    Ok(())
                },
                report: &mut |update| {
                    order.borrow_mut().push(format!("{update:?}"));
                    Ok(())
                },
                permit: &mut |_| Permit::Chosen("yes".to_owned()),
            },
        )
        .unwrap();
    assert_eq!(answer.stop, StopReason::Cancelled);
    drop(session);
    let wire = agent.join().unwrap();
    let cancels = wire
        .iter()
        .filter(|message| message["method"] == "session/cancel")
        .count();
    assert_eq!(cancels, 1, "the agent heard {wire:?}");
    assert_eq!(
        order.into_inner(),
        vec!["cancel booked".to_owned(), "Text(\"late\")".to_owned()]
    );
}
