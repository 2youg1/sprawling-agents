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

//! MCP handshake: initialize, initialized, ready.

use super::outbound::Outbound;
use super::tools::float_at;
use kernel::{AxCode, AxError, TimeoutMs};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// What a completed handshake learned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    /// The revision the far end agreed to, which is not always the one
    /// that was asked for.
    pub protocol_version: String,
    /// What the server calls itself. Recorded for diagnostics, never
    /// branched on: a name is not a capability.
    pub server: String,
}

/// Opens a connection: `initialize`, then `notifications/initialized`.
///
/// One authority for the lifecycle, above both transports. Before this
/// existed, both of them opened with `server/discover` - a method the
/// specification does not define - and a hosted server answered
/// `-32601: Method not found` to the first thing this city ever said to
/// it.
///
/// # Errors
/// Refuses a far end that will not answer `initialize`, and one whose
/// answer carries no protocol version.
pub fn handshake(
    out: &mut dyn Outbound,
    rpc: &mut Rpc,
    patience: TimeoutMs,
) -> Result<Handshake, AxError> {
    let answer = out.call(&rpc.initialize(), patience)?;
    let result = Rpc::read(&answer)?;
    let protocol_version = result
        .get("protocolVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "open an mcp connection",
                "the server's initialize answer names no protocol version",
            )
            .with_recovery("the far end is not an MCP server, or speaks a revision without one")
        })?
        .to_owned();
    let server = result
        .get("serverInfo")
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unnamed")
        .to_owned();
    // Required before any other request. A server that never receives it
    // is entitled to refuse everything that follows.
    out.notify(&Rpc::initialized(), patience)?;
    Ok(Handshake {
        protocol_version,
        server,
    })
}

/// Builds request lines and reads answer lines. Holds an id counter and
/// nothing else.
#[derive(Debug, Default)]
pub struct Rpc {
    next: u64,
}

impl Rpc {
    #[must_use]
    pub fn new() -> Rpc {
        Rpc { next: 0 }
    }

    fn mint(&mut self) -> u64 {
        self.next = self.next.saturating_add(1);
        self.next
    }

    /// One JSON-RPC request, on one line. The transport is line
    /// delimited and a message may not contain a newline, so the line is
    /// assembled here rather than left to a pretty printer.
    fn request(&mut self, method: &str, params: Value) -> String {
        let id = self.mint();
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":{},\"params\":{}}}",
            Value::String(method.to_owned()),
            params
        )
    }

    /// `initialize`: the first message of every connection.
    ///
    /// Carries the three things the specification requires - a protocol
    /// version, this client's capabilities, and who this client is. The
    /// capability object is empty on purpose: roots, sampling and
    /// elicitation are things a server may ask *us* for, and declaring a
    /// capability this city does not implement would invite a request it
    /// would then have to refuse.
    pub fn initialize(&mut self) -> String {
        self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "sprawling", "version": env!("CARGO_PKG_VERSION") },
            }),
        )
    }

    /// `notifications/initialized`: sent once the server has answered
    /// `initialize`, and required before any other request.
    ///
    /// A notification has no `id`, which is what tells the far end not
    /// to answer it.
    #[must_use]
    pub fn initialized() -> String {
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}".to_owned()
    }

    /// `tools/list`.
    pub fn list_tools(&mut self) -> String {
        self.request("tools/list", json!({}))
    }

    /// `tools/call`.
    ///
    /// # Errors
    /// Refuses arguments carrying a floating point number. Every call is
    /// written to the ledger, ledger payloads carry no floats, and a
    /// call that cannot be recorded is a call the city cannot replay.
    /// The refusal is scoped to the call rather than the tool: one
    /// badly-shaped argument is not evidence that the tool is unusable.
    pub fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<String, AxError> {
        if let Some(path) = float_at(arguments, String::new()) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "call an external tool",
                format!("a fractional number at `{path}` cannot be recorded"),
            )
            .with_recovery(
                "send the value as a string or an integer; every call is written to the ledger, \
                 and the ledger holds no floats",
            ));
        }
        Ok(self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        ))
    }

    /// Reads one answer line.
    ///
    /// # Errors
    /// Refuses a line that is not a JSON-RPC answer, and turns a
    /// server's error object into a refusal that keeps the server's own
    /// words.
    pub fn read(line: &str) -> Result<Value, AxError> {
        let value: Value = serde_json::from_str(line).map_err(|err| {
            AxError::failure(AxCode::WireMismatch, "read an mcp answer", err.to_string())
                .with_recovery("check the server's protocol revision")
        })?;
        let object = value.as_object().ok_or_else(|| {
            AxError::failure(AxCode::WireMismatch, "read an mcp answer", "not an object")
                .with_recovery("check the server's protocol revision")
        })?;
        if let Some(error) = object.get("error") {
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("no message");
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "call an external tool",
                format!("{code}: {message}"),
            )
            .with_recovery("the server refused; read its message before trying another shape"));
        }
        object.get("result").cloned().ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read an mcp answer",
                "neither result nor error",
            )
            .with_recovery("check the server's protocol revision")
        })
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
    use super::super::outbound::{EXTERNAL_CALL_PATIENCE, ScriptedOutbound};
    use super::*;

    #[test]
    fn a_request_is_one_line_with_no_newline_inside_it() {
        let mut rpc = Rpc::new();
        let line = rpc.list_tools();
        assert!(!line.contains('\n'));
        assert!(line.starts_with("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\""));
        assert!(
            rpc.initialize().contains("\"id\":2"),
            "ids are never reused"
        );
    }

    /// The opening message is the one the specification names. Before
    /// this, both transports opened with `server/discover`, which MCP
    /// does not define, and a hosted server answered `-32601: Method
    /// not found` to the first thing this city ever said to it.
    /// The opening message is the one the specification names. Before
    /// this, both transports opened with `server/discover`, which MCP
    /// does not define, and a hosted server answered `-32601: Method
    /// not found` to the first thing this city ever said to it.
    #[test]
    fn a_connection_opens_with_initialize_carrying_its_three_required_parts() {
        let line = Rpc::new().initialize();
        let sent: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(sent["method"], "initialize");
        assert_eq!(sent["params"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(sent["params"]["capabilities"].is_object());
        assert_eq!(sent["params"]["clientInfo"]["name"], "sprawling");
    }

    /// A notification has no `id`, which is what tells the far end not
    /// to answer it. An id here would leave a client waiting for a
    /// reply that a correct server will never send.
    /// A notification has no `id`, which is what tells the far end not
    /// to answer it. An id here would leave a client waiting for a
    /// reply that a correct server will never send.
    #[test]
    fn the_initialized_notification_carries_no_id() {
        let sent: Value = serde_json::from_str(&Rpc::initialized()).unwrap();
        assert_eq!(sent["method"], "notifications/initialized");
        assert!(sent.get("id").is_none());
    }

    /// The handshake reads what was negotiated rather than assuming its
    /// own version was accepted, and it sends the notification the
    /// specification requires before anything else may be asked.
    /// The handshake reads what was negotiated rather than assuming its
    /// own version was accepted, and it sends the notification the
    /// specification requires before anything else may be asked.
    #[test]
    fn a_handshake_records_what_was_negotiated_and_says_it_is_ready() {
        let mut server = ScriptedOutbound::new();
        server
            .answer(
                &Rpc::new().initialize(),
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\",\
                 \"capabilities\":{},\"serverInfo\":{\"name\":\"somebody\",\"version\":\"1\"}}}",
            )
            .unwrap();
        let mut rpc = Rpc::new();
        let opened = handshake(&mut server, &mut rpc, EXTERNAL_CALL_PATIENCE).unwrap();
        assert_eq!(opened.protocol_version, "2025-03-26");
        assert_eq!(opened.server, "somebody");
        assert!(
            server.missed().is_empty(),
            "the handshake asked for nothing that was not scripted: {:?}",
            server.missed()
        );
    }

    /// The defect that made the first real hosted server unusable: its
    /// results carry relevance scores, the ledger holds no floats, and
    /// four searches in a row came back as `E_INVALID_ARGS` on a number
    /// nobody in this city chose.
    /// A far end that answers `initialize` with no version is not
    /// speaking this protocol, and is refused rather than assumed.
    #[test]
    fn a_server_that_names_no_protocol_version_is_refused() {
        let mut server = ScriptedOutbound::new();
        server
            .answer(
                &Rpc::new().initialize(),
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}",
            )
            .unwrap();
        let err = handshake(&mut server, &mut Rpc::new(), EXTERNAL_CALL_PATIENCE).unwrap_err();
        assert_eq!(err.code(), &AxCode::WireMismatch);
    }

    #[test]
    fn an_answer_this_version_cannot_read_is_refused_rather_than_guessed() {
        for line in ["not json", "[]", "{\"jsonrpc\":\"2.0\",\"id\":1}"] {
            assert!(Rpc::read(line).is_err(), "{line}");
        }
    }
}
