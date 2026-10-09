// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One ACP session with an official harness, this city as the client
//! (`crates/agent_protocols/Spec.lean` §8-19).
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
//!
//! What a run over this session must keep - a report is never admitted
//! history, a halt is a cancel before anything else, a frozen run emits
//! nothing - is proved in `crates/agent_protocols/spec/Harness/Session.lean`.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use crate::mcp::{Heard, Lines};

mod introduced;
mod report;

use introduced::{AUTH_REQUIRED, auth_required, initialize_params};
pub use introduced::{AuthMethod, Introduced, LoginKind};
pub use report::{PermissionAsk, PermitKind, PermitOption, Update};
use report::{ask_of, update_of};

/// JSON-RPC's "method not found".
const METHOD_NOT_FOUND: i64 = -32_601;

/// The longest a halt waits for the harness to say something before the
/// session asks about it anyway (`crates/agent_protocols/Spec.lean` section 14).
const HALT_TICK_MS: u64 = 200;

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

/// What the caller is asked while a turn runs. Four callbacks rather
/// than a trait: there is one caller, and a test plays it with closures.
pub struct Listener<'a> {
    /// Whether a scope holding the room is halted. Asked before each
    /// message is handled and after each silent wait.
    pub halted: &'a mut dyn FnMut() -> bool,
    /// Called once, the first time `halted` answers yes, before
    /// `session/cancel` is sent: the caller books the cancel here.
    pub cancelling: &'a mut dyn FnMut() -> Result<(), AxError>,
    /// Every report, in the order the agent sent them.
    pub report: &'a mut dyn FnMut(Update) -> Result<(), AxError>,
    /// Every permission ask before the cancel; after it, each is
    /// answered `cancelled` without asking.
    pub permit: &'a mut dyn FnMut(&PermissionAsk) -> Permit,
}

/// How a turn ended, and what the agent said to the city in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub stop: StopReason,
    /// The turn's `agent_message_chunk`s, joined in order.
    pub text: String,
}

/// One open session.
pub struct AcpSession<W: Write> {
    lines: Lines,
    writer: W,
    /// The harness's name, for every refusal this session reports.
    name: String,
    next: u64,
    session: String,
    /// Whether `session/cancel` went out in this turn.
    cancelled: bool,
    /// What the agent said to the city in this turn so far.
    said: String,
    /// What the agent said about itself in `initialize`.
    introduced: Introduced,
}

impl<W: Write> AcpSession<W> {
    /// Opens a session in `cwd`: `initialize`, then `session/new`.
    ///
    /// # Errors
    /// The agent refused either request, answered in a shape this
    /// client cannot read, or closed its output before answering.
    pub fn open(lines: Lines, writer: W, name: &str, cwd: &Path) -> Result<Self, AxError> {
        let mut session = AcpSession {
            lines,
            writer,
            name: name.to_owned(),
            next: 0,
            session: String::new(),
            cancelled: false,
            said: String::new(),
            introduced: Introduced::default(),
        };
        // Nothing is halted before a turn, and nothing the agent says
        // while the session opens is a report.
        let (mut never, mut nothing) = (|| false, || Ok::<(), AxError>(()));
        let (mut ignored, mut refused) = (
            |_: Update| Ok::<(), AxError>(()),
            |_: &PermissionAsk| Permit::Cancelled,
        );
        let mut quiet = Listener {
            halted: &mut never,
            cancelling: &mut nothing,
            report: &mut ignored,
            permit: &mut refused,
        };
        let answer = session.request("initialize", initialize_params(), &mut quiet)?;
        session.introduced = introduced::introduced(&session.name, &answer)?;
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
            &mut quiet,
        )?;
        session.session = text_at(&opened, "sessionId")
            .ok_or_else(|| session.unreadable("session/new", "no sessionId"))?
            .to_owned();
        Ok(session)
    }

    /// What the agent said about itself when the session opened.
    #[must_use]
    pub fn introduced(&self) -> &Introduced {
        &self.introduced
    }

    /// Sends one prompt and reads the turn to its end, asking `listener`
    /// about a halt all along and handing it every report and ask.
    ///
    /// # Errors
    /// As [`AcpSession::open`], a stop reason this client does not know,
    /// and whatever `listener` refuses with.
    pub fn prompt(&mut self, text: &str, listener: &mut Listener<'_>) -> Result<Answer, AxError> {
        self.cancelled = false;
        self.said.clear();
        let params = json!({
            "sessionId": self.session,
            "prompt": [{ "type": "text", "text": text }]
        });
        let answer = self.request("session/prompt", params, listener)?;
        let stop = match text_at(&answer, "stopReason") {
            Some("end_turn") => StopReason::EndTurn,
            Some("max_tokens") => StopReason::MaxTokens,
            Some("max_turn_requests") => StopReason::MaxTurnRequests,
            Some("refusal") => StopReason::Refusal,
            Some("cancelled") => StopReason::Cancelled,
            Some(other) => {
                return Err(self.unreadable("session/prompt", &format!("stop reason {other}")));
            }
            None => return Err(self.unreadable("session/prompt", "no stopReason")),
        };
        Ok(Answer {
            stop,
            text: std::mem::take(&mut self.said),
        })
    }

    /// Sends one request and reads until its answer, serving what the
    /// agent says in between.
    fn request(
        &mut self,
        method: &str,
        params: Value,
        listener: &mut Listener<'_>,
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
            let heard = self.lines.next(Duration::from_millis(HALT_TICK_MS))?;
            // Before anything the agent said is handled, and after every
            // silent wait: a halt becomes a cancel ahead of the next
            // report (crates/agent_protocols/spec/Harness/Session.lean).
            self.heed_halt(listener)?;
            let line = match heard {
                Heard::Message(line) => line,
                Heard::Ended => return Err(self.ended(method)),
                Heard::Silent => continue,
            };
            let message: Value = serde_json::from_str(&line)
                .map_err(|_| self.unreadable(method, "a line that is not JSON"))?;
            match (
                message.get("id"),
                message.get("method").and_then(Value::as_str),
            ) {
                (Some(asked), Some(wanted)) => {
                    self.serve(asked.clone(), wanted, &message, listener)?
                }
                (None, Some("session/update")) => {
                    if let Some(update) = message.get("params").and_then(|p| p.get("update")) {
                        let update = update_of(update);
                        if let Update::Text(chunk) = &update {
                            self.said.push_str(chunk);
                        }
                        (listener.report)(update)?;
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

    /// Turns the first halt of a turn into a cancel: the caller books it,
    /// then `session/cancel` goes out. A later halt sends nothing, and
    /// outside a turn there is no session to cancel.
    fn heed_halt(&mut self, listener: &mut Listener<'_>) -> Result<(), AxError> {
        if self.cancelled || self.session.is_empty() || !(listener.halted)() {
            return Ok(());
        }
        (listener.cancelling)()?;
        self.cancelled = true;
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": { "sessionId": self.session }
        }))
    }

    /// Answers one request the agent made.
    fn serve(
        &mut self,
        asked: Value,
        wanted: &str,
        message: &Value,
        listener: &mut Listener<'_>,
    ) -> Result<(), AxError> {
        if wanted != "session/request_permission" {
            return self.send(&json!({
                "jsonrpc": "2.0",
                "id": asked,
                "error": { "code": METHOD_NOT_FOUND, "message": format!("{wanted} is not offered") }
            }));
        }
        let params = message.get("params").unwrap_or(&Value::Null);
        // After the cancel ACP wants every ask answered `cancelled`, and
        // the caller has nothing left to decide.
        let answered = if self.cancelled {
            Permit::Cancelled
        } else {
            (listener.permit)(&ask_of(params))
        };
        let outcome = match answered {
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
            if code == AUTH_REQUIRED {
                return Err(auth_required(
                    &self.name,
                    method,
                    &self.introduced.auth_methods,
                ));
            }
            return Err(AxError::failure(
                AxCode::Provider,
                format!("ask {} to {method}", self.name),
                format!("the agent refused with {code}: {said}"),
            )
            .with_recovery("read the agent's own message, then start the session again"));
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

#[cfg(test)]
mod tests;
