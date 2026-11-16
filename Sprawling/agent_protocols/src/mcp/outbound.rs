// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, TimeoutMs};
use serde_json::Value;

pub const EXTERNAL_CALL_PATIENCE: TimeoutMs = TimeoutMs(60_000);

pub trait Outbound: Send {

    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError>;

    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError>;
}

pub fn digits_for_floats(value: Value) -> Value {
    match value {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => Value::String(n.to_string()),
        Value::Array(items) => Value::Array(items.into_iter().map(digits_for_floats).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, held)| (key, digits_for_floats(held)))
                .collect(),
        ),
        other @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => other,
    }
}

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
        let tool = McpTool::new(
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
