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
    let mut input = Input::start(input, lines_read);
    let mut pending = VecDeque::new();
    let mut phase = Phase::Idle;
    let mut session_id = String::new();
    let mut cwd = PathBuf::new();
    let mut first = true;
    let mut next_session: u64 = 1;
    loop {
        let line = match pending.pop_front() {
            Some(line) => line,
            None => match input.next_line()? {
                Some(line) => line,
                None => return Ok(()),
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
                        let drained = run_turn_while_draining(input, turn_input, &mut turn)?;
                        input = drained.input;
                        pending.append(&mut { drained.pending });
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

enum InputEvent {
    Line(std::io::Result<String>),
    Eof,
    TurnDone,
}

struct Input {
    tx: mpsc::Sender<InputEvent>,
    rx: mpsc::Receiver<InputEvent>,
    eof: bool,
}

impl Input {
    fn start(reader: impl Read + Send + 'static, meter: Option<Arc<AtomicU64>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let lines = tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                if lines.send(InputEvent::Line(line)).is_err() {
                    return;
                }
                if let Some(meter) = &meter {
                    meter.fetch_add(1, Ordering::SeqCst);
                }
            }
            drop(lines.send(InputEvent::Eof));
        });
        Self { tx, rx, eof: false }
    }

    fn next_line(&mut self) -> Result<Option<String>> {
        while !self.eof {
            match self.rx.recv() {
                Ok(InputEvent::Line(line)) => {
                    return line.map(Some).map_err(|err| {
                        Diagnostic::new(
                            Code::Protocol,
                            "read an ACP line",
                            err.to_string(),
                            "keep stdin as UTF-8 JSON-RPC lines until the client closes it",
                        )
                        .into()
                    });
                }
                Ok(InputEvent::Eof) | Err(_) => self.eof = true,
                Ok(InputEvent::TurnDone) => continue,
            }
        }
        Ok(None)
    }
}

struct TurnDone(mpsc::Sender<InputEvent>);

impl Drop for TurnDone {
    fn drop(&mut self) {
        drop(self.0.send(InputEvent::TurnDone));
    }
}

struct TurnInput {
    input: Input,
    session: String,
    cancelled: Arc<AtomicBool>,
    pending: VecDeque<String>,
}

impl TurnInput {
    fn read_until_done(mut self) -> Result<Self> {
        while let Ok(event) = self.input.rx.recv() {
            if matches!(event, InputEvent::TurnDone) {
                break;
            }
            self.accept(event)?;
        }
        Ok(self)
    }

    fn accept(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Line(Ok(line)) => {
                note_cancel_or_queue(line, &self.session, &self.cancelled, &mut self.pending);
            }
            InputEvent::Line(Err(err)) => {
                return Err(Diagnostic::new(
                    Code::Protocol,
                    "read an ACP line during a turn",
                    err.to_string(),
                    "keep stdin open until the prompt response is written",
                )
                .into());
            }
            InputEvent::Eof => self.input.eof = true,
            InputEvent::TurnDone => {}
        }
        Ok(())
    }

    fn read_ready(&mut self) -> Result<()> {
        while let Ok(event) = self.input.rx.try_recv() {
            self.accept(event)?;
        }
        Ok(())
    }
}

struct DrainedTurn {
    result: Result<Outcome>,
    input: Input,
    pending: VecDeque<String>,
}

fn run_turn_while_draining(
    input: Input,
    turn_input: Turn,
    turn: &mut impl FnMut(Turn) -> Result<Outcome>,
) -> Result<DrainedTurn> {
    if input.eof {
        return Ok(DrainedTurn {
            result: turn(turn_input),
            input,
            pending: VecDeque::new(),
        });
    }
    let mut draining = TurnInput {
        input,
        session: turn_input.session_id.clone(),
        cancelled: Arc::clone(&turn_input.cancelled),
        pending: VecDeque::new(),
    };
    draining.read_ready()?;
    let done = TurnDone(draining.input.tx.clone());
    let drain = thread::spawn(move || draining.read_until_done());
    let result = turn(turn_input);
    drop(done);
    let mut drained = drain.join().map_err(|_| {
        Diagnostic::new(
            Code::Protocol,
            "join the ACP cancel drain",
            "the drain thread panicked",
            "restart mei --acp",
        )
    })??;
    drained.read_ready()?;
    Ok(DrainedTurn {
        result,
        input: drained.input,
        pending: drained.pending,
    })
}

fn note_cancel_or_queue(
    line: String,
    session_id: &str,
    cancelled: &AtomicBool,
    pending: &mut VecDeque<String>,
) {
    if line.trim().is_empty() {
        return;
    }
    if let Ok(message) = serde_json::from_str::<Value>(&line)
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
    pending.push_back(line);
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
