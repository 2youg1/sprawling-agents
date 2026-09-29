// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One ACP session with an official harness, this city as the client
//! (protocol-SPEC.md 8-19).
//!
//! JSON-RPC 2.0, one message per line, over whatever reader and writer
//! the caller hands in: a child's stdio in production, a pipe an agent
//! is played on in a test. The session opens with `initialize` and
//! `session/new`, and each prompt turn ends when the agent answers
//! `session/prompt` with a stop reason.
//!
//! **The city offers the harness no file system and no terminal.** The
//! harness runs its own tools; what reaches this module is what it
//! chooses to report and the permission it chooses to ask, so a session
//! records a harness and never governs it. A request this client never
//! advertised (`fs/*`, `terminal/*`) is answered `-32601` rather than
//! left unanswered, which would hang the agent.

use std::io::{BufRead, Write};
use std::path::Path;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use crate::mcp::{Received, read_one_message};

/// The protocol version this client speaks.
const PROTOCOL_VERSION: u64 = 1;

/// JSON-RPC's "method not found".
const METHOD_NOT_FOUND: i64 = -32_601;

/// Something the agent reported during a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// A chunk of the agent's answer.
    Text(String),
    /// A chunk of the agent's reasoning.
    Thought(String),
    /// A tool call the agent started, in its own words.
    ToolCall {
        id: String,
        title: String,
        kind: String,
    },
    /// A tool call's status moved: `in_progress`, `completed`, `failed`.
    ToolCallStatus { id: String, status: String },
    /// Any other variant, by name: a plan, a usage count, a mode.
    Other { variant: String },
}

/// A permission the agent asked for before running a tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    pub title: String,
    pub options: Vec<PermitOption>,
}

/// One answer the agent offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitOption {
    pub id: String,
    pub name: String,
    pub kind: PermitKind,
}

/// What an offered answer would do, as the agent labels it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

/// The caller's answer to a permission ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permit {
    /// One of the offered options, by id.
    Chosen(String),
    /// The turn is being cancelled; ACP requires this answer then.
    Cancelled,
}

/// Why the agent ended a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Cancelled,
}

/// One open session.
pub struct AcpSession<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    /// The harness's name, for every refusal this session reports.
    name: String,
    next: u64,
    session: String,
}

impl<R: BufRead, W: Write> AcpSession<R, W> {
    /// Opens a session in `cwd`: `initialize`, then `session/new`.
    ///
    /// # Errors
    /// The agent refused either request, answered in a shape this
    /// client cannot read, or closed its output before answering.
    pub fn open(reader: R, writer: W, name: &str, cwd: &Path) -> Result<Self, AxError> {
        let mut session = AcpSession {
            reader,
            writer,
            name: name.to_owned(),
            next: 0,
            session: String::new(),
        };
        let capabilities = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientCapabilities": {
                "fs": { "readTextFile": false, "writeTextFile": false },
                "terminal": false
            }
        });
        session.request("initialize", capabilities, &mut |_| {}, &mut |_| {
            Permit::Cancelled
        })?;
        let cwd = cwd.to_str().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "open an ACP session",
                "the working directory is not valid UTF-8",
            )
            .with_recovery("run the harness in a directory whose path is UTF-8")
        })?;
        let opened = session.request(
            "session/new",
            json!({ "cwd": cwd, "mcpServers": [] }),
            &mut |_| {},
            &mut |_| Permit::Cancelled,
        )?;
        session.session = text_at(&opened, "sessionId")
            .ok_or_else(|| session.unreadable("session/new", "no sessionId"))?
            .to_owned();
        Ok(session)
    }

    /// Sends one prompt and reads the turn to its end, handing every
    /// update to `updates` and every permission ask to `permit`.
    ///
    /// # Errors
    /// As [`AcpSession::open`], and a stop reason this client does not
    /// know.
    pub fn prompt(
        &mut self,
        text: &str,
        updates: &mut dyn FnMut(Update),
        permit: &mut dyn FnMut(&PermissionAsk) -> Permit,
    ) -> Result<StopReason, AxError> {
        let params = json!({
            "sessionId": self.session,
            "prompt": [{ "type": "text", "text": text }]
        });
        let answer = self.request("session/prompt", params, updates, permit)?;
        match text_at(&answer, "stopReason") {
            Some("end_turn") => Ok(StopReason::EndTurn),
            Some("max_tokens") => Ok(StopReason::MaxTokens),
            Some("max_turn_requests") => Ok(StopReason::MaxTurnRequests),
            Some("refusal") => Ok(StopReason::Refusal),
            Some("cancelled") => Ok(StopReason::Cancelled),
            Some(other) => Err(self.unreadable("session/prompt", &format!("stop reason {other}"))),
            None => Err(self.unreadable("session/prompt", "no stopReason")),
        }
    }

    /// Sends one request and reads until its answer, serving what the
    /// agent says in between.
    fn request(
        &mut self,
        method: &str,
        params: Value,
        updates: &mut dyn FnMut(Update),
        permit: &mut dyn FnMut(&PermissionAsk) -> Permit,
    ) -> Result<Value, AxError> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "number an ACP request",
                self.name.clone(),
            )
            .with_recovery("start a new session")
        })?;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let line = match read_one_message(&mut self.reader, &self.name)? {
                Received::Message(line) => line,
                Received::EndOfInput => return Err(self.ended(method)),
            };
            let message: Value = serde_json::from_str(&line)
                .map_err(|_| self.unreadable(method, "a line that is not JSON"))?;
            match (
                message.get("id"),
                message.get("method").and_then(Value::as_str),
            ) {
                (Some(asked), Some(wanted)) => {
                    self.serve(asked.clone(), wanted, &message, permit)?
                }
                (None, Some("session/update")) => {
                    if let Some(update) = message.get("params").and_then(|p| p.get("update")) {
                        updates(update_of(update));
                    }
                }
                (None, Some(_)) => {}
                (Some(answered), None) if answered.as_u64() == Some(id) => {
                    return self.result_of(method, &message);
                }
                (Some(_) | None, None) => {}
            }
        }
    }

    /// Answers one request the agent made.
    fn serve(
        &mut self,
        asked: Value,
        wanted: &str,
        message: &Value,
        permit: &mut dyn FnMut(&PermissionAsk) -> Permit,
    ) -> Result<(), AxError> {
        if wanted != "session/request_permission" {
            return self.send(&json!({
                "jsonrpc": "2.0",
                "id": asked,
                "error": { "code": METHOD_NOT_FOUND, "message": format!("{wanted} is not offered") }
            }));
        }
        let params = message.get("params").unwrap_or(&Value::Null);
        let outcome = match permit(&ask_of(params)) {
            Permit::Chosen(option) => json!({ "outcome": "selected", "optionId": option }),
            Permit::Cancelled => json!({ "outcome": "cancelled" }),
        };
        self.send(&json!({ "jsonrpc": "2.0", "id": asked, "result": { "outcome": outcome } }))
    }

    fn result_of(&self, method: &str, message: &Value) -> Result<Value, AxError> {
        if let Some(error) = message.get("error") {
            let code = error
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let said = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            return Err(AxError::failure(
                AxCode::Provider,
                format!("ask {} to {method}", self.name),
                format!("the harness refused with {code}: {said}"),
            )
            .with_recovery("sign in inside the harness, then start the session again"));
        }
        message
            .get("result")
            .cloned()
            .ok_or_else(|| self.unreadable(method, "an answer with neither result nor error"))
    }

    fn send(&mut self, message: &Value) -> Result<(), AxError> {
        let mut line = message.to_string();
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .and_then(|()| self.writer.flush())
            .map_err(|err| {
                AxError::failure(
                    AxCode::Provider,
                    format!("write to {}", self.name),
                    err.to_string(),
                )
                .with_recovery("the harness process has gone; start the session again")
            })
    }

    fn unreadable(&self, method: &str, what: &str) -> AxError {
        AxError::failure(
            AxCode::WireMismatch,
            format!("read {}'s answer to {method}", self.name),
            what.to_owned(),
        )
        .with_recovery("update the harness to a release that speaks ACP version 1")
    }

    fn ended(&self, method: &str) -> AxError {
        AxError::failure(
            AxCode::Provider,
            format!("read {}'s answer to {method}", self.name),
            "the harness closed its output before answering",
        )
        .effect_unknown()
        .with_recovery(
            "the harness may already have acted on the prompt; look at the room's files \
             before sending it again",
        )
    }
}

fn text_at<'v>(value: &'v Value, key: &str) -> Option<&'v str> {
    value.get(key).and_then(Value::as_str)
}

/// One `session/update` payload as this client reads it. A variant it
/// has no reading for keeps its name, so nothing an agent reports is
/// dropped without a trace.
fn update_of(update: &Value) -> Update {
    let variant = text_at(update, "sessionUpdate").unwrap_or_default();
    let chunk = || {
        update
            .get("content")
            .and_then(|content| text_at(content, "text"))
            .unwrap_or_default()
            .to_owned()
    };
    let field = |key: &str| text_at(update, key).unwrap_or_default().to_owned();
    match variant {
        "agent_message_chunk" => Update::Text(chunk()),
        "agent_thought_chunk" => Update::Thought(chunk()),
        "tool_call" => Update::ToolCall {
            id: field("toolCallId"),
            title: field("title"),
            kind: field("kind"),
        },
        "tool_call_update" => Update::ToolCallStatus {
            id: field("toolCallId"),
            status: field("status"),
        },
        other => Update::Other {
            variant: other.to_owned(),
        },
    }
}

/// One `session/request_permission` as this client reads it. An option
/// whose kind is not one of ACP's four is left out: offering a person a
/// choice whose effect nobody can name would be a guess.
fn ask_of(params: &Value) -> PermissionAsk {
    let title = params
        .get("toolCall")
        .and_then(|call| text_at(call, "title"))
        .unwrap_or_default()
        .to_owned();
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let kind = match text_at(option, "kind")? {
                        "allow_once" => PermitKind::AllowOnce,
                        "allow_always" => PermitKind::AllowAlways,
                        "reject_once" => PermitKind::RejectOnce,
                        "reject_always" => PermitKind::RejectAlways,
                        _ => return None,
                    };
                    Some(PermitOption {
                        id: text_at(option, "optionId")?.to_owned(),
                        name: text_at(option, "name").unwrap_or_default().to_owned(),
                        kind,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    PermissionAsk { title, options }
}

#[cfg(test)]
mod tests;
