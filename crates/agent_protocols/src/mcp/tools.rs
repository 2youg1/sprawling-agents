// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! MCP tools: listing, naming, calling.
//!
//! Two servers offering one verb stay two tools, a constructed tool is
//! outside a confidential building and has a deadline, and a reply's
//! `isError` and `_meta` decide how it is read: the properties are proved
//! in `crates/agent_protocols/spec/Mcp/Tools.lean`.

use std::sync::Mutex;

use super::handshake::Rpc;
use super::outbound::{EXTERNAL_CALL_PATIENCE, Outbound};
use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, ServerLabel, Temporal, TimeoutMs,
    Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

/// The `_meta` key a server sets on an `isError` result when part of the
/// call may already have taken effect (`crates/agent_protocols/Spec.lean` §8-1c).
/// `desktop/src/refusal.rs` quotes it, and `xtask guard` compares the two.
pub(crate) const EFFECT_META_KEY: &str = "sprawling/effect-unknown";

/// How much of a server's own failure text reaches the subject: a refusal
/// and its recovery fit, a stack trace or a whole page does not
/// (`crates/agent_protocols/Spec.lean` §8-1c).
pub(crate) const ERROR_TEXT_CAP_BYTES: usize = 4_096;

/// What a caller does after a tool reported its own failure.
const FAILED_RECOVERY: &str = "the server said what failed and what to do instead in the words \
                               above; follow them rather than repeating the call unchanged";

/// What a caller does after a failure the server marked as possibly
/// half done.
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

/// One tool a server offers, under both of its names.
///
/// The two travel together because the function that derives one from
/// the other is the only place that knows both: a caller that had to
/// pair a list of registrations with a list of remote names by position
/// would be one reordering away from calling the wrong tool.
pub struct Listed {
    /// The name the server knows it by, which is what goes back out.
    pub remote: String,
    /// The name this city knows it by, which is what the model sees.
    pub meta: ToolMeta,
}

/// Turns a `tools/list` result into the registrations a catalog admits.
///
/// # Errors
/// Refuses a result whose shape this version does not read, and a tool
/// whose name cannot be spelled after prefixing.
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
                // Every external call leaves this process. Naming the
                // connector is what routes it to the gate that can say
                // no, and what tells that gate where it goes: the
                // destination is this server, on every call, so there is
                // nothing here for a model to fill in.
                effect: Effect::Connector {
                    label: server.clone(),
                },
                cost_tier: CostTier::Heavy,
                timeout: Some(EXTERNAL_CALL_PATIENCE),
                render: RenderIntent::Generic,
                // The far end is a live service, so the answer is about
                // now.
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

/// A tool a server offers, ready to be registered on the tool seam.
pub struct McpTool {
    meta: ToolMeta,
    remote: String,
    patience: TimeoutMs,
    /// The request id and the connection it numbers are one lock: a
    /// reply is read off the same connection the request went out on,
    /// so two calls on one server take turns.
    link: Mutex<Link>,
}

struct Link {
    rpc: Rpc,
    outbound: Box<dyn Outbound>,
}

impl McpTool {
    /// # Errors
    /// Refuses to exist inside a confidential building. That mark means
    /// what happens here leaves no trace outside the run, and an
    /// outbound call is the largest trace there is — so the refusal is
    /// at construction, where there is nothing yet to leak.
    ///
    /// Refuses a registration that declares no timeout: an outbound tool
    /// with no deadline is a tool that can hang a run, and the run has
    /// no second way to notice.
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

    /// The server-side name, which is what goes back out on the wire.
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

/// A `tools/call` result the server marked `isError`, read as the failure
/// it is: the text blocks in order are the subject, and a `_meta` that
/// marks the effect unknown turns it into an outcome nobody can see.
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

/// The server's text, cut at a character boundary once it passes
/// `ERROR_TEXT_CAP_BYTES`, with the length of what was cut.
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

    /// A server's results carry fractional numbers such as relevance
    /// scores, and the ledger holds no floats, so an answer keeps each
    /// such number as its own digits instead of failing the call.
    #[test]
    fn a_servers_fractional_numbers_survive_as_their_own_digits() {
        let answered = serde_json::json!({
            "results": [{ "title": "a page", "score": 1249.4, "rank": 1 }],
            "cost": { "total": 0.005 }
        });
        let recordable = digits_for_floats(answered);
        assert_eq!(recordable["results"][0]["score"], "1249.4");
        assert_eq!(recordable["cost"]["total"], "0.005");
        // Integers are numbers still: this is not a blanket stringifier.
        assert_eq!(recordable["results"][0]["rank"], 1);
        assert_eq!(recordable["results"][0]["title"], "a page");
        // And the whole thing is now something the ledger will take.
        let Value::Object(map) = recordable else {
            panic!("an object stays an object");
        };
        Payload::new(map).expect("a payload with no floats left in it");
    }

    /// A far end that answers `initialize` with no version is not
    /// speaking this protocol, and is refused rather than assumed.
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
        // The tool is still usable with a shape that can be recorded.
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

    /// The tool that answers `line` with `answer`, and the call that
    /// reaches it, both through the doors production uses.
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

    /// A tool that says it failed has failed: the model reads the
    /// server's own words as an error, not as a strange answer.
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

    /// A failure the server marks as possibly half done is not a call to
    /// repeat, and says so through the retry the city reads.
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
