// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::answer::Answer;
use crate::platform;
use crate::refusal::{Refusal, RefusalCode};
use crate::rpc::{self, PROTOCOL_VERSION, Request};
use crate::scope::{Reach, Scope};
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Fresh,
    Initializing,
    Ready,
}

impl Phase {

    fn opened(self) -> Result<Phase, Refusal> {
        match self {
            Phase::Fresh => Err(unfinished("finish the handshake")),
            Phase::Initializing | Phase::Ready => Ok(Phase::Ready),
        }
    }
}

pub(crate) struct Server {
    scope: Scope,
    phase: Phase,
    desk: platform::Desk,
}

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
            desk: platform::Desk::new(),
        }
    }

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
                self.phase = self.phase.opened()?;
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

    fn handshaken(&self, action: &str) -> Result<(), Refusal> {
        match self.phase {
            Phase::Ready => Ok(()),
            Phase::Fresh | Phase::Initializing => Err(unfinished(action)),
        }
    }

    fn call(&mut self, params: &Value) -> Result<Value, Refusal> {
        let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "use the desktop",
                "the call names no tool",
                "send `name` alongside `arguments`, as `tools/call` describes",
            )
        })?;
        let tool = tools::ToolName::parse(name).ok_or_else(|| {
            Refusal::new(
                RefusalCode::ToolUnknown,
                "use the desktop",
                format!("`{name}` is not a tool this server offers"),
                "read `tools/list`; this server offers six tools and no others",
            )
        })?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        Ok(match self.carried_out(tool, &arguments) {
            Ok(answer) => answer.as_result(),
            Err(refusal) => refusal.as_tool_result(),
        })
    }

    fn carried_out(&mut self, tool: tools::ToolName, arguments: &Value) -> Result<Answer, Refusal> {
        let admitted = self.scope.admits(&Reach {
            tool,
            title: arguments.get("title").and_then(Value::as_str),
            process: arguments.get("process").and_then(Value::as_str),
        })?;

        self.desk.perform(tool, arguments, &admitted)
    }
}

fn unfinished(action: &str) -> Refusal {
    Refusal::new(
        RefusalCode::GateDenied,
        action,
        "this connection has not finished its handshake",
        "send `initialize`, then the `notifications/initialized` notification, \
         before anything else",
    )
}

fn listing() -> Vec<Value> {
    tools::table()
        .into_iter()
        .map(|card| {
            json!({
                "name": card.name.as_str(),
                "description": card.description,
                "inputSchema": card.schema,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
