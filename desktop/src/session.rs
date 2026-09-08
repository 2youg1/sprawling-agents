// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One connection: a blocking read loop, one reply per request.
//!
//! No async runtime. A server that answers one caller over one pair of
//! pipes has one thing to wait for, and a reactor to wait for one thing
//! is a dependency bought with nothing.
//!
//! The handshake order is enforced rather than assumed. The
//! specification puts `initialize` first and the
//! `notifications/initialized` notification second, and a server that
//! lets work through before them turns a client's ordering mistake into
//! a failure somewhere else, wearing some other shape.

use crate::platform;
use crate::refusal::{Refusal, RefusalCode};
use crate::rpc::{self, PROTOCOL_VERSION, Request};
use crate::scope::{Reach, Scope};
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;

/// Where this connection has got to. Exhaustive, because "has the
/// handshake finished" has three answers and not two: nothing said yet,
/// `initialize` answered, and the notification received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Fresh,
    Initializing,
    Ready,
}

/// One connection, and the scope it is bounded by.
pub(crate) struct Server {
    scope: Scope,
    phase: Phase,
}

/// Serves one connection over this process's own pipes, under the scope
/// file at `scope_path`.
///
/// This is the whole public surface of this package. Everything else is
/// reached through the protocol, which is the point of a server.
///
/// # Errors
/// Returns what the operating system said when a read or a write failed.
/// End of input is not a failure: it is how a connection closes.
pub fn serve_stdio(scope_path: Option<&Path>) -> std::io::Result<()> {
    let mut server = Server::new(Scope::read(scope_path));
    let input = std::io::stdin();
    let mut output = std::io::stdout();
    server.serve(input.lock(), &mut output)
}

impl Server {
    pub(crate) fn new(scope: Scope) -> Server {
        Server {
            scope,
            phase: Phase::Fresh,
        }
    }

    /// Reads lines until the caller closes the pipe, answering each one.
    ///
    /// # Errors
    /// Returns what the operating system said when a read or a write
    /// failed.
    pub(crate) fn serve(
        &mut self,
        input: impl BufRead,
        output: &mut impl Write,
    ) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(answer) = self.answer(&line) {
                writeln!(output, "{answer}")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    /// Answers one line. `None` is the answer to a notification, and it
    /// means nothing is written: a line written back for a notification
    /// would be read as the answer to the next request.
    pub(crate) fn answer(&mut self, line: &str) -> Option<String> {
        let request = match rpc::read(line) {
            Ok(request) => request,
            Err(refusal) => return Some(rpc::error_line(None, &refusal)),
        };
        let outcome = self.dispatch(&request);
        let id = request.id.as_ref()?;
        Some(match outcome {
            Ok(result) => rpc::result_line(id, result),
            Err(refusal) => rpc::error_line(Some(id), &refusal),
        })
    }

    /// One method, one answer.
    fn dispatch(&mut self, request: &Request) -> Result<Value, Refusal> {
        match request.method.as_str() {
            "initialize" => {
                self.phase = Phase::Initializing;
                Ok(json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": "sprawling-desktop",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                }))
            }
            "notifications/initialized" => {
                self.phase = Phase::Ready;
                Ok(json!({}))
            }
            "ping" => Ok(json!({})),
            "tools/list" => {
                self.handshaken("list the desktop tools")?;
                Ok(json!({ "tools": listing() }))
            }
            "tools/call" => {
                self.handshaken("use the desktop")?;
                self.call(&request.params)
            }
            unknown => Err(Refusal::new(
                RefusalCode::ToolUnknown,
                "answer a request",
                format!("`{unknown}` is not a method this server offers"),
                "this server answers initialize, notifications/initialized, tools/list, \
                 tools/call and ping",
            )),
        }
    }

    /// # Errors
    /// Refuses anything asked before the handshake finished.
    fn handshaken(&self, action: &str) -> Result<(), Refusal> {
        match self.phase {
            Phase::Ready => Ok(()),
            Phase::Fresh | Phase::Initializing => Err(Refusal::new(
                RefusalCode::GateDenied,
                action,
                "this connection has not finished its handshake",
                "send `initialize`, then the `notifications/initialized` notification, \
                 before anything else",
            )),
        }
    }

    /// One `tools/call`: named, admitted, then carried out.
    fn call(&self, params: &Value) -> Result<Value, Refusal> {
        let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "use the desktop",
                "the call names no tool",
                "send `name` alongside `arguments`, as `tools/call` describes",
            )
        })?;
        if tools::card(name).is_none() {
            return Err(Refusal::new(
                RefusalCode::ToolUnknown,
                "use the desktop",
                format!("`{name}` is not a tool this server offers"),
                "read `tools/list`; this server offers six tools and no others",
            ));
        }
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        self.scope.admits(&Reach {
            tool: name,
            title: arguments.get("title").and_then(Value::as_str),
            process: arguments.get("process").and_then(Value::as_str),
        })?;
        platform::perform(name, &arguments)
    }
}

/// The tool table as `tools/list` publishes it.
fn listing() -> Vec<Value> {
    tools::table()
        .into_iter()
        .map(|card| {
            json!({
                "name": card.name,
                "description": card.description,
                "inputSchema": card.schema,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
