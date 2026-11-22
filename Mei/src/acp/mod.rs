// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result};

mod codec;
use codec::{
    initialize_result, open_session, prompt_params, write_error, write_result, write_update,
};

const PARSE_ERROR: i64 = -32_700;
const INVALID_REQUEST: i64 = -32_600;
const METHOD_NOT_FOUND: i64 = -32_601;
const INVALID_PARAMS: i64 = -32_602;
const INTERNAL_ERROR: i64 = -32_603;

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
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text = block.get("text").and_then(Value::as_str)?;
                if !text.is_empty() {
                    parts.push(text.to_owned());
                }
            }
            Some("resource_link") => {
                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                let uri = block.get("uri").and_then(Value::as_str).unwrap_or("");
                if name.is_empty() && uri.is_empty() {
                    continue;
                }
                parts.push(format!("resource_link name={name} uri={uri}"));
            }
            _ => {}
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
    let (tx, mut rx) = mpsc::channel::<std::io::Result<String>>();
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
        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(err) => {
                write_error(&mut output, &Value::Null, PARSE_ERROR, err.to_string())?;
                continue;
            }
        };
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
                        let turn_input = Turn {
                            session_id: session_id.clone(),
                            cwd: cwd.clone(),
                            prompt,
                            first,
                            cancelled: Arc::clone(&cancelled),
                        };
                        let drained = run_turn_while_draining(TurnDrain {
                            turn_input,
                            turn: &mut turn,
                            rx,
                            session_id: &session_id,
                            cancelled: &cancelled,
                        })?;
                        rx = drained.rx;
                        pending.append(&mut { drained.pending });
                        drain_during_turn(&rx, &mut pending, &session_id, &cancelled)?;
                        let result = drained.result;
                        match result {
                            Ok(outcome) => {
                                let stop = finish(cancelled.load(Ordering::SeqCst), outcome.stop);
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
                            Err(error) => {
                                if cancelled.load(Ordering::SeqCst) {
                                    eprintln!("{error}");
                                    write_result(
                                        &mut output,
                                        id,
                                        json!({ "stopReason": StopReason::Cancelled.as_str() }),
                                    )?;
                                    first = false;
                                } else {
                                    write_error(
                                        &mut output,
                                        id,
                                        INTERNAL_ERROR,
                                        error.to_string(),
                                    )?;
                                }
                            }
                        }
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

struct TurnDrain<'a, F> {
    turn_input: Turn,
    turn: &'a mut F,
    rx: mpsc::Receiver<std::io::Result<String>>,
    session_id: &'a str,
    cancelled: &'a Arc<AtomicBool>,
}

struct DrainedTurn {
    result: Result<Outcome>,
    rx: mpsc::Receiver<std::io::Result<String>>,
    pending: VecDeque<String>,
}

fn run_turn_while_draining<F>(parts: TurnDrain<'_, F>) -> Result<DrainedTurn>
where
    F: FnMut(Turn) -> Result<Outcome>,
{
    let TurnDrain {
        turn_input,
        turn,
        rx,
        session_id,
        cancelled,
    } = parts;
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let (back_tx, back_rx) = mpsc::channel::<
        std::result::Result<(mpsc::Receiver<std::io::Result<String>>, VecDeque<String>), String>,
    >();
    let session = session_id.to_owned();
    let flag = Arc::clone(cancelled);
    let drain = thread::spawn(move || {
        let mut pending = VecDeque::new();
        let mut rx = rx;
        let outcome = loop {
            if done_rx.try_recv().is_ok() {
                break Ok((rx, pending));
            }
            match rx.recv_timeout(Duration::from_millis(5)) {
                Ok(Ok(line)) => note_cancel_or_queue(&line, &session, &flag, &mut pending),
                Ok(Err(err)) => break Err(err.to_string()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break Ok((rx, pending)),
            }
        };
        let _ = back_tx.send(outcome);
    });
    let result = turn(turn_input);
    let _ = done_tx.send(());
    let reclaimed = back_rx.recv().map_err(|_| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "reclaim ACP stdin after a turn",
            "the cancel drain thread exited without returning the reader",
            "restart `mei --acp`",
        ))
    })?;
    let _ = drain.join();
    let (rx, pending) = reclaimed.map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "read an ACP line during a turn",
            err,
            "keep stdin open until the prompt response is written",
        ))
    })?;
    Ok(DrainedTurn {
        result,
        rx,
        pending,
    })
}

fn note_cancel_or_queue(
    line: &str,
    session_id: &str,
    cancelled: &AtomicBool,
    pending: &mut VecDeque<String>,
) {
    if line.trim().is_empty() {
        return;
    }
    if let Ok(message) = serde_json::from_str::<Value>(line)
        && message.get("method").and_then(Value::as_str) == Some("session/cancel")
    {
        let matches = message
            .get("params")
            .and_then(|params| params.get("sessionId"))
            .and_then(Value::as_str)
            .is_some_and(|id| id == session_id);
        if matches {
            cancelled.store(true, Ordering::SeqCst);
        }
        return;
    }
    pending.push_back(line.to_owned());
}

fn drain_during_turn(
    rx: &mpsc::Receiver<std::io::Result<String>>,
    pending: &mut VecDeque<String>,
    session_id: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    loop {
        match rx.try_recv() {
            Ok(Ok(line)) => note_cancel_or_queue(&line, session_id, cancelled, pending),
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
