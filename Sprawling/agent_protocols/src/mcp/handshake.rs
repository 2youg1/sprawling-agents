// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::outbound::Outbound;
use super::tools::float_at;
use kernel::{AxCode, AxError, TimeoutMs};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {

    pub protocol_version: String,

    pub server: String,
}

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

    out.notify(&Rpc::initialized(), patience)?;
    Ok(Handshake {
        protocol_version,
        server,
    })
}

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

    fn request(&mut self, method: &str, params: Value) -> String {
        let id = self.mint();
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":{},\"params\":{}}}",
            Value::String(method.to_owned()),
            params
        )
    }

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

    #[must_use]
    pub fn initialized() -> String {
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}".to_owned()
    }

    pub fn list_tools(&mut self) -> String {
        self.request("tools/list", json!({}))
    }

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

    #[test]
    fn a_connection_opens_with_initialize_carrying_its_three_required_parts() {
        let line = Rpc::new().initialize();
        let sent: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(sent["method"], "initialize");
        assert_eq!(sent["params"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(sent["params"]["capabilities"].is_object());
        assert_eq!(sent["params"]["clientInfo"]["name"], "sprawling");
    }

    #[test]
    fn the_initialized_notification_carries_no_id() {
        let sent: Value = serde_json::from_str(&Rpc::initialized()).unwrap();
        assert_eq!(sent["method"], "notifications/initialized");
        assert!(sent.get("id").is_none());
    }

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

    struct Counting {
        calls: u32,
        notifications: u32,
    }

    impl Outbound for Counting {
        fn call(&mut self, line: &str, _patience: TimeoutMs) -> Result<String, AxError> {
            self.calls = self.calls.saturating_add(1);
            let answer = if line.contains("\"initialize\"") {
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\
                 \"serverInfo\":{\"name\":\"counter\",\"version\":\"1\"}}}"
            } else {
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}"
            };
            Ok(answer.to_owned())
        }

        fn notify(&mut self, _line: &str, _patience: TimeoutMs) -> Result<(), AxError> {
            self.notifications = self.notifications.saturating_add(1);
            Ok(())
        }
    }

    #[test]
    fn opening_one_connection_costs_two_round_trips_and_one_notification() {
        let mut server = Counting {
            calls: 0,
            notifications: 0,
        };
        let rounds = 100u32;
        for _ in 0..rounds {
            let mut rpc = Rpc::new();
            let opened = handshake(&mut server, &mut rpc, EXTERNAL_CALL_PATIENCE).unwrap();
            assert_eq!(opened.server, "counter");
            let listed = server
                .call(&rpc.list_tools(), EXTERNAL_CALL_PATIENCE)
                .unwrap();
            Rpc::read(&listed).unwrap();
        }
        assert_eq!(server.calls, 2 * rounds, "initialize and tools/list");
        assert_eq!(server.notifications, rounds, "notifications/initialized");
    }
}
