// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Mutex;

use super::handshake::Rpc;
use super::outbound::{EXTERNAL_CALL_PATIENCE, Outbound};
use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, ServerLabel, Temporal, TimeoutMs,
    Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

pub const EFFECT_META_KEY: &str = "sprawling/effect-unknown";

pub(crate) const ERROR_TEXT_CAP_BYTES: usize = 4_096;

const FAILED_RECOVERY: &str = "the server said what failed and what to do instead in the words \
                               above; follow them rather than repeating the call unchanged";

const UNKNOWN_RECOVERY: &str = "the server says part of this call may already have taken \
                                effect; look at what it acted on before calling again, and do \
                                not repeat it unchanged";

pub(crate) fn float_at(value: &Value, path: String) -> Option<String> {
    match value {
        Value::Number(number) => {
            if number.is_f64() && number.as_i64().is_none() && number.as_u64().is_none() {
                Some(if path.is_empty() {
                    "(the argument itself)".to_owned()
                } else {
                    path
                })
            } else {
                None
            }
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(index, item)| float_at(item, format!("{path}[{index}]"))),
        Value::Object(map) => map.iter().find_map(|(key, item)| {
            let next = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            float_at(item, next)
        }),
        Value::Null | Value::Bool(_) | Value::String(_) => None,
    }
}

pub struct Listed {
    pub remote: String,

    pub meta: ToolMeta,
}

pub fn tools_from(server: &ServerLabel, result: &Value) -> Result<Vec<Listed>, AxError> {
    let listed = result
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read an mcp tool list",
                "no `tools` array",
            )
            .with_recovery("check the server's protocol revision")
        })?;
    let mut out = Vec::new();
    for entry in listed {
        let raw = entry.get("name").and_then(Value::as_str).ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read an mcp tool list",
                "a tool without a name",
            )
            .with_recovery("check the server's protocol revision")
        })?;
        let name = ToolName::parse(&format!("{}_{}", server.as_str(), sanitise(raw)))?;
        let disclosure = entry
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("an external tool this server offers")
            .to_owned();
        let params = entry
            .get("inputSchema")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        out.push(Listed {
            remote: raw.to_owned(),
            meta: ToolMeta {
                name,
                disclosure,
                params: Payload::new(params)?,

                effect: Effect::Connector {
                    label: server.clone(),
                },
                cost_tier: CostTier::Heavy,
                timeout: Some(EXTERNAL_CALL_PATIENCE),
                render: RenderIntent::Generic,

                temporal: Temporal::Timestamped,
            },
        });
    }
    Ok(out)
}

fn sanitise(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub struct McpTool {
    meta: ToolMeta,
    remote: String,
    patience: TimeoutMs,

    link: Mutex<Link>,
}

struct Link {
    rpc: Rpc,
    outbound: Box<dyn Outbound>,
}

impl McpTool {
    pub fn new(
        meta: ToolMeta,
        remote: String,
        outbound: Box<dyn Outbound>,
        confidential: bool,
    ) -> Result<McpTool, AxError> {
        if confidential {
            return Err(AxError::failure(
                AxCode::GateDenied,
                "offer an external tool",
                format!("{remote}: this building is confidential"),
            )
            .with_recovery("do this work in a building that is allowed to reach the network"));
        }
        let patience = meta.timeout.ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "offer an external tool",
                format!("{remote}: no timeout declared"),
            )
            .with_recovery("register the tool with a timeout; an outbound call needs a deadline")
        })?;
        Ok(McpTool {
            meta,
            remote,
            patience,
            link: Mutex::new(Link {
                rpc: Rpc::new(),
                outbound,
            }),
        })
    }

    #[must_use]
    pub fn remote(&self) -> &str {
        &self.remote
    }
}

impl Tool for McpTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "call an external tool",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let arguments = Value::Object(call.args.as_map().clone());
        let answer = {
            let mut link = self.link.lock().map_err(|_| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "call an external tool",
                    format!(
                        "{}: an earlier call died holding the connection",
                        self.remote
                    ),
                )
                .with_recovery("dispatch the work again; the next run connects afresh")
            })?;
            let line = link.rpc.call_tool(&self.remote, &arguments)?;
            link.outbound.call(&line, self.patience)?
        };
        let result = super::outbound::digits_for_floats(Rpc::read(&answer)?);
        if result.get("isError") == Some(&Value::Bool(true)) {
            return Err(reported_failure(&self.remote, &result));
        }
        let map = match result {
            Value::Object(map) => map,
            other @ (Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_)) => {
                let mut wrapped = Map::new();
                wrapped.insert("result".to_owned(), other);
                wrapped
            }
        };
        Ok(ToolOutcome {
            result: Payload::new(map)?,
            attachments: Vec::new(),
        })
    }
}

fn reported_failure(remote: &str, result: &Value) -> AxError {
    let blocks = result
        .get("content")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let is_text = |block: &&Value| block.get("type").and_then(Value::as_str) == Some("text");
    let text = blocks
        .iter()
        .filter(is_text)
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let left_out = match blocks.iter().filter(|block| !is_text(block)).count() {
        0 => String::new(),
        blocks => format!("; {blocks} non-text blocks left out"),
    };
    let subject = if text.is_empty() {
        format!("{remote} reported a failure and gave no text{left_out}")
    } else {
        format!("{remote} reported a failure: {}{left_out}", capped(&text))
    };
    let unknown = result
        .get("_meta")
        .and_then(|meta| meta.get(EFFECT_META_KEY))
        .is_some_and(|flag| flag != &Value::Bool(false));
    if unknown {
        AxError::failure(AxCode::ToolOutcomeUnknown, "call an external tool", subject)
            .effect_unknown()
            .with_recovery(UNKNOWN_RECOVERY)
    } else {
        AxError::failure(AxCode::ToolUnavailable, "call an external tool", subject)
            .with_recovery(FAILED_RECOVERY)
    }
}

fn capped(text: &str) -> String {
    let end = text.floor_char_boundary(ERROR_TEXT_CAP_BYTES);
    match text.get(..end) {
        Some(kept) if end < text.len() => format!(
            "{kept} \u{2026} ({} more bytes left out)",
            text.len().saturating_sub(end)
        ),
        Some(_) | None => text.to_owned(),
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
    use super::super::outbound::{ScriptedOutbound, digits_for_floats};
    use super::*;
    use kernel::ServerLabel;
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
    fn a_servers_fractional_numbers_survive_as_their_own_digits() {
        let answered = serde_json::json!({
            "results": [{ "title": "a page", "score": 1249.4, "rank": 1 }],
            "cost": { "total": 0.005 }
        });
        let recordable = digits_for_floats(answered);
        assert_eq!(recordable["results"][0]["score"], "1249.4");
        assert_eq!(recordable["cost"]["total"], "0.005");

        assert_eq!(recordable["results"][0]["rank"], 1);
        assert_eq!(recordable["results"][0]["title"], "a page");

        let Value::Object(map) = recordable else {
            panic!("an object stays an object");
        };
        Payload::new(map).expect("a payload with no floats left in it");
    }

    #[test]
    fn two_servers_offering_the_same_verb_stay_two_tools() {
        let one = tools_from(&label(), &listing()).unwrap();
        let other = ServerLabel::parse("desk").unwrap();
        let two = tools_from(&other, &listing()).unwrap();
        assert_eq!(one[0].meta.name.as_str(), "apps_github_create_issue");
        assert_eq!(two[0].meta.name.as_str(), "desk_github_create_issue");
        assert_ne!(one[0].meta.name, two[0].meta.name);
    }

    #[test]
    fn an_external_tool_declares_itself_as_leaving_the_machine() {
        let tools = tools_from(&label(), &listing()).unwrap();
        assert_eq!(tools[0].meta.effect, Effect::Connector { label: label() });
        assert_eq!(tools[0].meta.temporal, Temporal::Timestamped);
        assert_eq!(tools[1].meta.name.as_str(), "apps_gmail_send");
    }

    #[test]
    fn a_fractional_argument_is_refused_by_position_and_only_that_call() {
        let mut rpc = Rpc::new();
        let err = rpc
            .call_tool("x", &json!({ "amount": 1.5, "note": "ok" }))
            .unwrap_err();
        assert!(err.subject().contains("amount"), "{}", err.subject());
        assert!(err.recovery().contains("ledger"));

        assert!(rpc.call_tool("x", &json!({ "amount": 2 })).is_ok());
        let nested = rpc
            .call_tool("x", &json!({ "a": { "b": [1, 2.25] } }))
            .unwrap_err();
        assert!(nested.subject().contains("a.b[1]"), "{}", nested.subject());
    }

    #[test]
    fn a_servers_refusal_keeps_the_servers_own_words() {
        let err = Rpc::read(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32602,\"message\":\"no such repo\"}}",
        )
        .unwrap_err();
        assert!(err.subject().contains("no such repo"));
        assert_eq!(err.code(), &AxCode::ToolUnavailable);
    }

    fn answering(answer: &str) -> (McpTool, ToolCall) {
        let arguments = json!({ "title": "kiln" });
        let line = Rpc::new()
            .call_tool("GITHUB_CREATE_ISSUE", &arguments)
            .unwrap();
        let mut scripted = ScriptedOutbound::new();
        scripted.answer(&line, answer).unwrap();
        let meta = tools_from(&label(), &listing()).unwrap().remove(0).meta;
        let name = meta.name.clone();
        let tool = McpTool::new(
            meta,
            "GITHUB_CREATE_ISSUE".to_owned(),
            Box::new(scripted),
            false,
        )
        .unwrap();
        let call = ToolCall {
            id: "tu_1".to_owned(),
            name,
            args: Payload::new(arguments.as_object().unwrap().clone()).unwrap(),
        };
        (tool, call)
    }

    #[test]
    fn a_tool_that_reports_its_own_failure_is_a_failure_in_the_servers_words() {
        let (tool, call) = answering(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"E_GATE_DENIED: cannot use the desktop \u{2014} no\\ninstead: add it\"}],\"isError\":true}}",
        );
        assert_eq!(
            tool.invoke(&call),
            Err(AxError::failure(
                AxCode::ToolUnavailable,
                "call an external tool",
                "GITHUB_CREATE_ISSUE reported a failure: E_GATE_DENIED: cannot use the desktop \u{2014} no\ninstead: add it",
            )
            .with_recovery(FAILED_RECOVERY))
        );
    }

    #[test]
    fn a_failure_whose_effect_is_unknown_says_so() {
        let (tool, call) = answering(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"E_GATE_DENIED: cannot use the desktop \u{2014} no\\ninstead: add it\"}],\"isError\":true,\"_meta\":{\"sprawling/effect-unknown\":true}}}",
        );
        assert_eq!(
            tool.invoke(&call),
            Err(AxError::failure(
                AxCode::ToolOutcomeUnknown,
                "call an external tool",
                "GITHUB_CREATE_ISSUE reported a failure: E_GATE_DENIED: cannot use the desktop \u{2014} no\ninstead: add it",
            )
            .effect_unknown()
            .with_recovery(UNKNOWN_RECOVERY))
        );
    }

    #[test]
    fn a_confidential_building_cannot_hold_an_outbound_tool_at_all() {
        let listed = tools_from(&label(), &listing()).unwrap().remove(0);
        let meta = listed.meta;
        let err = McpTool::new(
            meta,
            "GITHUB_CREATE_ISSUE".to_owned(),
            Box::new(ScriptedOutbound::new()),
            true,
        )
        .err()
        .expect("a confidential building refuses an outbound tool");
        assert_eq!(err.code(), &AxCode::GateDenied);
        assert!(err.recovery().contains("reach the network"));
    }
}
