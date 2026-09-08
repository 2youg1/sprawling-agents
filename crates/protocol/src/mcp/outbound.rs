// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reaching a Model Context Protocol server.
//!
//! The city does not know any particular server. Which one a building
//! talks to is that building's configuration; what this module knows is
//! the protocol, and the protocol is the same whether the far end is a
//! hosted catalogue of a thousand applications or a script somebody
//! wrote this morning.
//!
//! Two things about the current revision shape the code. The list of
//! tools may not vary per connection, which is the same rule as freezing
//! a run's tool table — the two arrived from opposite directions and
//! agree, so the tool table is read once and frozen with the run. And
//! every connection opens with the lifecycle the specification defines:
//! `initialize`, then a `notifications/initialized` notification, before
//! any other request. That is why the seam has two methods rather than
//! one — a notification is a message with no answer, and pretending it
//! has one is how a client ends up waiting for a 202 with no body.
//!
//! Everything a server returns is other people's text. It lands on the
//! same tool seam as the local tools, so it enters the taint ring the
//! same way, and there is no unwrapping face here.

//! MCP outbound: one line per request.

use kernel::{AxCode, AxError, TimeoutMs};
use serde_json::Value;

pub const EXTERNAL_CALL_PATIENCE: TimeoutMs = TimeoutMs(60_000);

/// The wire this module speaks over. Adapters: a stdio subprocess in the
/// binary, and [`ScriptedOutbound`] for replay.
///
/// One method, and it is synchronous, because a tool call is a question
/// with an answer. Where the bytes go and how long they take belongs to
/// the adapter. `Send`, because the tool that holds one is `Send`
/// (sprawling-SPEC 8-44).
pub trait Outbound: Send {
    /// Sends one JSON-RPC line and returns the line that answered it,
    /// giving up after `patience`.
    ///
    /// The deadline is an argument rather than a property of the
    /// transport because it is the tool's declared timeout: a
    /// `TimeoutMs` in a registration promises the call can be given up
    /// on, and a promise nobody executes is decoration.
    ///
    /// # Errors
    /// Transport failures and the deadline. A server's own refusal comes
    /// back as a JSON-RPC error inside a successful exchange.
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError>;

    /// Sends one JSON-RPC notification and does not wait for an answer,
    /// because a notification has none.
    ///
    /// Separate from [`Outbound::call`] rather than folded into it: over
    /// HTTP a notification is answered with 202 and an empty body, so a
    /// caller that read it as a request would refuse a correct server
    /// for having said nothing.
    ///
    /// # Errors
    /// Transport failures and the deadline.
    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError>;
}

/// The protocol revision this client speaks.
///
/// A constant that follows the outside world: the specification names
/// the revision by date, and a server answering with a different one has
pub fn digits_for_floats(value: Value) -> Value {
    match value {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => Value::String(n.to_string()),
        Value::Array(items) => Value::Array(items.into_iter().map(digits_for_floats).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, held)| (key, digits_for_floats(held)))
                .collect(),
        ),
        other => other,
    }
}

/// The second adapter: answers somebody already collected.
///
/// Keyed by the request's method and params rather than by arrival
/// order, so a run that legitimately reorders two independent calls
/// still replays.
#[derive(Debug, Default)]
pub struct ScriptedOutbound {
    answers: std::collections::BTreeMap<String, String>,
    missed: Vec<String>,
}

impl ScriptedOutbound {
    #[must_use]
    pub fn new() -> ScriptedOutbound {
        ScriptedOutbound::default()
    }

    /// Records the answer for a request line, ignoring its id.
    ///
    /// # Errors
    /// Refuses a request line it cannot read, since a key derived from
    /// an unreadable line would never match anything.
    pub fn answer(&mut self, request: &str, answer: &str) -> Result<(), AxError> {
        self.answers.insert(key_of(request)?, answer.to_owned());
        Ok(())
    }

    #[must_use]
    pub fn missed(&self) -> &[String] {
        &self.missed
    }
}

fn key_of(request: &str) -> Result<String, AxError> {
    let value: Value = serde_json::from_str(request).map_err(|err| {
        AxError::failure(AxCode::WireMismatch, "key an mcp request", err.to_string())
            .with_recovery("record a well-formed JSON-RPC line")
    })?;
    let method = value
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    Ok(format!("{method} {params}"))
}

impl Outbound for ScriptedOutbound {
    // A recorded answer is instant, so the deadline has nothing to bound
    // here; it stays in the signature because the seam, not the adapter,
    // is what the tool's declared timeout travels through.
    // A notification is recorded as having been sent and nothing more:
    // there is no answer to script.
    fn notify(&mut self, line: &str, _patience: TimeoutMs) -> Result<(), AxError> {
        key_of(line).map(|_| ())
    }

    fn call(&mut self, line: &str, _patience: TimeoutMs) -> Result<String, AxError> {
        let key = key_of(line)?;
        match self.answers.get(&key) {
            Some(answer) => Ok(answer.clone()),
            None => {
                self.missed.push(key.clone());
                Err(
                    AxError::failure(AxCode::ToolUnavailable, "replay an mcp call", key)
                        .with_recovery("record this call against the real server before replaying"),
                )
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::handshake::Rpc;
    use super::super::tools::{McpTool, tools_from};
    use super::*;
    use kernel::Tool;
    use kernel::{Payload, ServerLabel, ToolCall};
    use serde_json::json;
    fn label() -> ServerLabel {
        ServerLabel::parse("apps").unwrap()
    }
    fn listing() -> Value {
        json!({ "tools": [
            { "name": "GITHUB_CREATE_ISSUE", "description": "open an issue",
              "inputSchema": { "type": "object" } },
            { "name": "gmail.send", "description": "send mail" },
        ] })
    }

    #[test]
    fn a_recorded_call_replays_to_the_same_answer() {
        let mut rpc = Rpc::new();
        let request = rpc
            .call_tool("GITHUB_CREATE_ISSUE", &json!({ "title": "kiln" }))
            .unwrap();
        let mut scripted = ScriptedOutbound::new();
        scripted
            .answer(
                &request,
                "{\"jsonrpc\":\"2.0\",\"id\":9,\"result\":{\"number\":41}}",
            )
            .unwrap();

        let listed = tools_from(&label(), &listing()).unwrap().remove(0);
        let meta = listed.meta;
        let name = meta.name.clone();
        let mut tool = McpTool::new(
            meta,
            "GITHUB_CREATE_ISSUE".to_owned(),
            Box::new(scripted),
            false,
        )
        .map_err(|err| format!("{err}"))
        .unwrap();
        let outcome = tool
            .invoke(&ToolCall {
                id: "tu_1".to_owned(),
                name,
                args: Payload::new(json!({ "title": "kiln" }).as_object().unwrap().clone())
                    .unwrap(),
            })
            .unwrap();
        assert_eq!(
            outcome
                .result
                .as_map()
                .get("number")
                .and_then(Value::as_u64),
            Some(41)
        );
    }

    #[test]
    fn an_outbound_tool_without_a_deadline_is_refused_at_construction() {
        let mut meta = tools_from(&label(), &listing()).unwrap().remove(0).meta;
        assert_eq!(meta.timeout, Some(EXTERNAL_CALL_PATIENCE));
        meta.timeout = None;
        let err = McpTool::new(
            meta,
            "GITHUB_CREATE_ISSUE".to_owned(),
            Box::new(ScriptedOutbound::new()),
            false,
        )
        .err()
        .expect("a tool that cannot be given up on is not offered");
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("deadline"));
    }
}
