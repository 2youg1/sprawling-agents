// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result};

const PROTOCOL_VERSION: u64 = 1;
const METHOD_NOT_FOUND: i64 = -32_601;
const INVALID_PARAMS: i64 = -32_602;
const INVALID_REQUEST: i64 = -32_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    Cancelled,
}

impl StopReason {
    fn as_str(self) -> &'static str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Turn {
    pub session_id: String,
    pub cwd: PathBuf,
    pub prompt: String,
    pub first: bool,
    pub cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub stop: StopReason,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Introduced,
    Session,
}

pub fn prompt_text(blocks: &[Value]) -> Option<String> {
    let mut parts = Vec::new();
    for block in blocks {
        if block.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        let text = block.get("text").and_then(Value::as_str)?;
        if !text.is_empty() {
            parts.push(text.to_owned());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

pub fn finish(saw_cancel: bool, produced: StopReason) -> StopReason {
    if saw_cancel {
        StopReason::Cancelled
    } else {
        produced
    }
}

pub fn serve(
    input: impl Read + Send + 'static,
    output: impl Write,
    turn: impl FnMut(Turn) -> Result<Outcome>,
) -> Result<()> {
    serve_with_meter(input, output, turn, None)
}

pub fn serve_with_meter(
    input: impl Read + Send + 'static,
    mut output: impl Write,
    mut turn: impl FnMut(Turn) -> Result<Outcome>,
    lines_read: Option<Arc<AtomicU64>>,
) -> Result<()> {
    let (tx, rx) = mpsc::channel::<std::io::Result<String>>();
    let meter = lines_read.clone();
    thread::spawn(move || {
        let reader = BufReader::new(input);
        for line in reader.lines() {
            if tx.send(line).is_err() {
                break;
            }
            if let Some(meter) = &meter {
                meter.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    let mut pending = VecDeque::new();
    let mut phase = Phase::Idle;
    let mut session_id = String::new();
    let mut cwd = PathBuf::new();
    let mut first = true;
    let mut next_session: u64 = 1;
    loop {
        let line = match pending.pop_front() {
            Some(line) => line,
            None => match rx.recv() {
                Ok(Ok(line)) => line,
                Ok(Err(err)) => {
                    return Err(Error::Diagnostic(Diagnostic::new(
                        Code::Protocol,
                        "read an ACP line",
                        err.to_string(),
                        "keep stdin as UTF-8 JSON-RPC lines until the client closes it",
                    )));
                }
                Err(_) => return Ok(()),
            },
        };
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = serde_json::from_str(&line).map_err(|err| {
            Error::Diagnostic(Diagnostic::new(
                Code::Protocol,
                "parse an ACP line",
                err.to_string(),
                "send one JSON-RPC object per line as ACP version 1 requires",
            ))
        })?;
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match (id.as_ref(), method) {
            (Some(id), Some("initialize")) if phase == Phase::Idle => {
                write_result(&mut output, id, initialize_result())?;
                phase = Phase::Introduced;
            }
            (Some(id), Some("session/new")) if phase == Phase::Introduced => {
                match open_session(&params, &mut next_session) {
                    Ok((id_value, path)) => {
                        session_id = id_value.clone();
                        cwd = path;
                        first = true;
                        write_result(&mut output, id, json!({ "sessionId": id_value }))?;
                        phase = Phase::Session;
                    }
                    Err(error) => write_error(&mut output, id, INVALID_PARAMS, error)?,
                }
            }
            (Some(id), Some("session/prompt")) if phase == Phase::Session => {
                match prompt_params(&params, &session_id) {
                    Ok(prompt) => {
                        let cancelled = Arc::new(AtomicBool::new(false));
                        let result = turn(Turn {
                            session_id: session_id.clone(),
                            cwd: cwd.clone(),
                            prompt,
                            first,
                            cancelled: Arc::clone(&cancelled),
                        });
                        drain_during_turn(&rx, &mut pending, &session_id, &cancelled)?;
                        let outcome = match result {
                            Ok(outcome) => outcome,
                            Err(error) => {
                                write_error(&mut output, id, -32_603, error.to_string())?;
                                continue;
                            }
                        };
                        let stop =
                            finish(cancelled.load(Ordering::SeqCst), outcome.stop);
                        if !outcome.text.is_empty() {
                            write_update(
                                &mut output,
                                &session_id,
                                json!({
                                    "sessionUpdate": "agent_message_chunk",
                                    "content": { "type": "text", "text": outcome.text }
                                }),
                            )?;
                        }
                        write_result(
                            &mut output,
                            id,
                            json!({ "stopReason": stop.as_str() }),
                        )?;
                        first = false;
                    }
                    Err(error) => write_error(&mut output, id, INVALID_PARAMS, error)?,
                }
            }
            (None, Some("session/cancel")) if phase == Phase::Session => {}
            (Some(id), Some(method)) => {
                write_error(
                    &mut output,
                    id,
                    METHOD_NOT_FOUND,
                    format!("method `{method}` is not offered in this phase"),
                )?;
            }
            (None, Some(_)) => {}
            (Some(id), None) => {
                write_error(
                    &mut output,
                    id,
                    INVALID_REQUEST,
                    "a request must carry a method".to_owned(),
                )?;
            }
            (None, None) => {}
        }
    }
}

fn drain_during_turn(
    rx: &mpsc::Receiver<std::io::Result<String>>,
    pending: &mut VecDeque<String>,
    session_id: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    loop {
        match rx.try_recv() {
            Ok(Ok(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(message) = serde_json::from_str::<Value>(&line)
                    && message.get("method").and_then(Value::as_str) == Some("session/cancel")
                {
                    let matches = message
                        .get("params")
                        .and_then(|params| params.get("sessionId"))
                        .and_then(Value::as_str)
                        .is_none_or(|id| id == session_id);
                    if matches {
                        cancelled.store(true, Ordering::SeqCst);
                    }
                    continue;
                }
                pending.push_back(line);
            }
            Ok(Err(err)) => {
                return Err(Error::Diagnostic(Diagnostic::new(
                    Code::Protocol,
                    "read an ACP line during a turn",
                    err.to_string(),
                    "keep stdin open until the prompt response is written",
                )));
            }
            Err(mpsc::TryRecvError::Empty) => return Ok(()),
            Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": {}
        },
        "agentInfo": {
            "name": "mei",
            "title": "Mei",
            "version": env!("CARGO_PKG_VERSION")
        },
        "authMethods": []
    })
}

fn open_session(params: &Value, next: &mut u64) -> std::result::Result<(String, PathBuf), String> {
    let cwd = params
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or_else(|| "session/new requires a UTF-8 cwd string".to_owned())?;
    if cwd.is_empty() {
        return Err("session/new cwd must not be empty".to_owned());
    }
    let path = PathBuf::from(cwd);
    if !path.is_absolute() {
        return Err("session/new cwd must be an absolute path".to_owned());
    }
    if !path.is_dir() {
        return Err(format!(
            "session/new cwd is not a directory: {}",
            path.display()
        ));
    }
    let id = *next;
    *next = next
        .checked_add(1)
        .ok_or_else(|| "too many ACP sessions in one process; restart mei --acp".to_owned())?;
    Ok((format!("mei_{id}"), path))
}

fn prompt_params(params: &Value, session_id: &str) -> std::result::Result<String, String> {
    let asked = params
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "session/prompt requires sessionId".to_owned())?;
    if asked != session_id {
        return Err(format!("unknown sessionId `{asked}`"));
    }
    let prompt = params
        .get("prompt")
        .and_then(Value::as_array)
        .ok_or_else(|| "session/prompt requires a prompt array".to_owned())?;
    prompt_text(prompt)
        .ok_or_else(|| "session/prompt needs at least one non-empty text block".to_owned())
}

fn write_result(output: &mut impl Write, id: &Value, result: Value) -> Result<()> {
    write_message(output, &json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn write_error(output: &mut impl Write, id: &Value, code: i64, message: String) -> Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    )
}

fn write_update(output: &mut impl Write, session_id: &str, update: Value) -> Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": { "sessionId": session_id, "update": update }
        }),
    )
}

fn write_message(output: &mut impl Write, message: &Value) -> Result<()> {
    let mut line = serde_json::to_string(message).map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "encode an ACP message",
            err.to_string(),
            "keep ACP payloads within JSON object limits",
        ))
    })?;
    line.push('\n');
    output.write_all(line.as_bytes()).map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "write an ACP message",
            err.to_string(),
            "keep the client stdout pipe open for JSON-RPC replies",
        ))
    })?;
    output.flush().map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "flush ACP stdout",
            err.to_string(),
            "keep the client stdout pipe open for JSON-RPC replies",
        ))
    })
}

pub fn enter_cwd(cwd: &Path) -> Result<()> {
    std::env::set_current_dir(cwd).map_err(|err| {
        Error::Diagnostic(
            Diagnostic::new(
                Code::Protocol,
                "enter the ACP session cwd",
                err.to_string(),
                "pass a readable absolute directory as session/new cwd",
            )
            .with_path(cwd.display().to_string()),
        )
    })
}
