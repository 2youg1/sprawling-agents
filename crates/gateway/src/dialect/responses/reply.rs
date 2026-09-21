// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The responses face's answer, in both directions.

use kernel::{
    AxCode, AxError, ChatResponse, ContentBlock, ModelUsage, StopReason, Tokens, ToolName,
};
use serde_json::{Value, json};

use crate::mismatch::{as_str, mismatch, mismatch_found, payload_from, require, tokens_or_zero};

/// The provider took the request, answered 200, and put nothing in
/// it: the envelope is this face's and `output` is null. The chat face
/// answers a ceiling above what the model allows the same way, so this
/// gets the same reading and the same way out.
fn empty_answer() -> AxError {
    AxError::failure(
        AxCode::Provider,
        "read the provider's answer",
        "the provider accepted the request and returned no answer at all",
    )
    .retriable()
    .with_recovery(
        "lower this model's max output tokens - a ceiling above what the model allows is \
         answered this way rather than refused - then dispatch again",
    )
}

/// Which of the output items this city reads.
///
/// Named rather than matched as text at the use site, so the arm that
/// passes an item over is a decision with a name instead of a
/// wildcard. Built-in tool calls this city never asked for, and items
/// added after this build, are [`Item::Unread`] \— not a failure,
/// because every field this city needs sits beside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Reasoning,
    Message,
    FunctionCall,
    Unread,
}

impl Item {
    fn of(word: &str) -> Item {
        match word {
            "reasoning" => Item::Reasoning,
            "message" => Item::Message,
            "function_call" => Item::FunctionCall,
            _ => Item::Unread,
        }
    }
}

/// The reasoning one `reasoning` output item states in words. Only
/// the summary is readable: the reasoning itself travels encrypted.
fn reasoning_text(item: &Value) -> String {
    let mut out = String::new();
    let Some(parts) = item.get("summary").and_then(Value::as_array) else {
        return out;
    };
    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str)
            && !text.is_empty()
        {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(text);
        }
    }
    out
}

/// The text one `message` item holds, in the order it was written.
fn message_text(item: &Value) -> String {
    let mut out = String::new();
    let Some(parts) = item.get("content").and_then(Value::as_array) else {
        return out;
    };
    for part in parts {
        if part.get("type").and_then(Value::as_str) == Some("output_text")
            && let Some(text) = part.get("text").and_then(Value::as_str)
        {
            out.push_str(text);
        }
    }
    out
}

/// One `function_call` output item as a canonical block.
fn tool_use(item: &Value, at: usize) -> Result<ContentBlock, AxError> {
    let path = format!("response.output[{at}]");
    let name_raw = as_str(require(item, &path, "name")?, &format!("{path}.name"))?;
    let name = ToolName::parse(name_raw)
        .map_err(|_| mismatch_found(&format!("{path}.name"), "not a tool name", name_raw))?;
    let arguments = as_str(
        require(item, &path, "arguments")?,
        &format!("{path}.arguments"),
    )?;
    let parsed: Value = serde_json::from_str(arguments)
        .map_err(|err| mismatch(&format!("{path}.arguments"), &err.to_string()))?;
    Ok(ContentBlock::ToolUse {
        id: as_str(require(item, &path, "call_id")?, &format!("{path}.call_id"))?.to_owned(),
        name,
        input: payload_from(&parsed, &format!("{path}.arguments"))?,
    })
}

/// Why the model stopped, from the answer as a whole.
///
/// This face reports no per-message finish reason: it reports a status
/// for the response and, when that status is `incomplete`, the
/// reason. A call that produced a tool use is a tool use, whatever
/// the status says, because the caller has a call to run either way.
fn stop_of(wire: &Value, made_a_call: bool) -> Result<StopReason, AxError> {
    if made_a_call {
        return Ok(StopReason::ToolUse);
    }
    let status = as_str(require(wire, "response", "status")?, "response.status")?;
    match status {
        "completed" => Ok(StopReason::EndTurn),
        "incomplete" => {
            let reason = wire
                .get("incomplete_details")
                .and_then(|details| details.get("reason"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if reason == "max_output_tokens" {
                Ok(StopReason::MaxTokens)
            } else {
                Err(mismatch_found(
                    "response.incomplete_details.reason",
                    "the answer stopped for a reason this build does not read",
                    reason,
                ))
            }
        }
        other => Err(mismatch_found(
            "response.status",
            "the answer is not one this build can read",
            other,
        )),
    }
}

pub(crate) fn response_from(wire: &Value) -> Result<ChatResponse, AxError> {
    let offered = require(wire, "response", "output")?;
    if offered.is_null() {
        return Err(empty_answer());
    }
    let output = offered
        .as_array()
        .ok_or_else(|| mismatch("response.output", "expected array"))?;
    let mut content = Vec::new();
    let mut made_a_call = false;
    for (at, item) in output.iter().enumerate() {
        let path = format!("response.output[{at}]");
        match Item::of(as_str(
            require(item, &path, "type")?,
            &format!("{path}.type"),
        )?) {
            // First in the array and first in the record: it is what
            // the model did before the answer it produced.
            Item::Reasoning => {
                let thinking = reasoning_text(item);
                if !thinking.is_empty() {
                    content.push(ContentBlock::Thinking {
                        thinking,
                        // This face carries none this city may send
                        // back, and an invented one is checked and
                        // refused by the provider that issued it.
                        signature: String::new(),
                    });
                }
            }
            Item::Message => {
                let text = message_text(item);
                if !text.is_empty() {
                    content.push(ContentBlock::Text { text });
                }
            }
            Item::FunctionCall => {
                made_a_call = true;
                content.push(tool_use(item, at)?);
            }
            Item::Unread => {}
        }
    }
    let stop = stop_of(wire, made_a_call)?;
    let usage_value = require(wire, "response", "usage")?;
    let details = usage_value.get("input_tokens_details");
    let cache_read = details
        .map(|held| tokens_or_zero(held, "cached_tokens", "response.usage.input_tokens_details"))
        .transpose()?
        .unwrap_or(Tokens::new(0));
    let cache_write = details
        .map(|held| {
            tokens_or_zero(
                held,
                "cache_write_tokens",
                "response.usage.input_tokens_details",
            )
        })
        .transpose()?
        .unwrap_or(Tokens::new(0));
    Ok(ChatResponse {
        content,
        stop,
        usage: ModelUsage {
            input_tokens: tokens_or_zero(usage_value, "input_tokens", "response.usage")?,
            output_tokens: tokens_or_zero(usage_value, "output_tokens", "response.usage")?,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
        },
    })
}

pub(crate) fn response_wire(resp: &ChatResponse) -> Result<Value, AxError> {
    let mut output = Vec::new();
    let mut parts = Vec::new();
    for block in &resp.content {
        match block {
            ContentBlock::Thinking { thinking, .. } => output.push(json!({
                "type": "reasoning",
                "id": "",
                "summary": [ { "type": "summary_text", "text": thinking } ],
            })),
            ContentBlock::Text { text } => parts.push(json!({
                "type": "output_text", "text": text, "annotations": [], "logprobs": [],
            })),
            ContentBlock::ToolUse { .. }
            | ContentBlock::Image(_)
            | ContentBlock::ToolResult { .. }
            | ContentBlock::RedactedThinking { .. } => {}
        }
    }
    if !parts.is_empty() {
        output.push(json!({
            "type": "message", "role": "assistant", "status": "completed", "content": parts,
        }));
    }
    for block in &resp.content {
        if let ContentBlock::ToolUse { id, name, input } = block {
            let arguments = serde_json::to_string(input)
                .map_err(|err| mismatch("tool_use.input", &err.to_string()))?;
            output.push(json!({
                "type": "function_call",
                "call_id": id,
                "name": name.as_str(),
                "arguments": arguments,
            }));
        }
    }
    let mut root = json!({
        "output": output,
        "usage": {
            "input_tokens": resp.usage.input_tokens.get(),
            "output_tokens": resp.usage.output_tokens.get(),
            "input_tokens_details": {
                "cached_tokens": resp.usage.cache_read_tokens.get(),
                "cache_write_tokens": resp.usage.cache_write_tokens.get(),
            },
        },
    });
    let status = match resp.stop {
        StopReason::EndTurn | StopReason::ToolUse => "completed",
        StopReason::MaxTokens => "incomplete",
    };
    if let Some(map) = root.as_object_mut() {
        map.insert("status".to_owned(), Value::String(status.to_owned()));
        if resp.stop == StopReason::MaxTokens {
            map.insert(
                "incomplete_details".to_owned(),
                json!({ "reason": "max_output_tokens" }),
            );
        }
    }
    Ok(root)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::Payload;

    fn call(name: &str, id: &str) -> ContentBlock {
        let mut args = serde_json::Map::new();
        args.insert("path".to_owned(), Value::String("notes.md".to_owned()));
        ContentBlock::ToolUse {
            id: id.to_owned(),
            name: ToolName::parse(name).unwrap(),
            input: Payload::new(args).unwrap(),
        }
    }

    /// What this city writes down, written onto the wire and read
    /// back, is what it wrote down.
    #[test]
    fn an_answer_survives_the_round_trip_through_this_face() {
        let held = ChatResponse {
            content: vec![
                ContentBlock::Thinking {
                    thinking: "two parts".to_owned(),
                    signature: String::new(),
                },
                ContentBlock::Text {
                    text: "here it is".to_owned(),
                },
                call("read", "fc_1"),
            ],
            stop: StopReason::ToolUse,
            usage: ModelUsage {
                input_tokens: Tokens::new(11),
                output_tokens: Tokens::new(7),
                cache_read_tokens: Tokens::new(3),
                cache_write_tokens: Tokens::new(2),
            },
        };
        let back = response_from(&response_wire(&held).unwrap()).unwrap();
        assert_eq!(back, held);
    }

    /// The ceiling stopping an answer is a status and a reason on this
    /// face, not a finish reason on a choice.
    #[test]
    fn an_answer_the_ceiling_stopped_reads_back_as_truncated() {
        let held = ChatResponse {
            content: vec![ContentBlock::Text {
                text: "half of it".to_owned(),
            }],
            stop: StopReason::MaxTokens,
            usage: ModelUsage {
                input_tokens: Tokens::new(1),
                output_tokens: Tokens::new(1),
                cache_read_tokens: Tokens::new(0),
                cache_write_tokens: Tokens::new(0),
            },
        };
        let wire = response_wire(&held).unwrap();
        assert_eq!(wire["status"], "incomplete");
        assert_eq!(wire["incomplete_details"]["reason"], "max_output_tokens");
        assert_eq!(response_from(&wire).unwrap().stop, StopReason::MaxTokens);
    }

    /// Told apart from a shape this build cannot read, because what a
    /// person does about it is different.
    #[test]
    fn an_answer_with_nothing_in_it_says_what_to_change() {
        let refused = response_from(&json!({ "output": Value::Null })).unwrap_err();
        assert_eq!(refused.code(), &AxCode::Provider);
        assert!(refused.recovery().contains("max output tokens"));
    }

    /// Passed over rather than refused: every field this city needs
    /// sits beside it.
    #[test]
    fn an_output_item_this_build_does_not_read_is_passed_over() {
        let wire = json!({
            "status": "completed",
            "output": [
                { "type": "web_search_call", "id": "ws_1", "status": "completed" },
                { "type": "message", "role": "assistant",
                  "content": [ { "type": "output_text", "text": "found it" } ] },
            ],
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        });
        let read = response_from(&wire).unwrap();
        assert_eq!(
            read.content,
            vec![ContentBlock::Text {
                text: "found it".to_owned()
            }]
        );
        assert_eq!(read.stop, StopReason::EndTurn);
    }

    /// Tool arguments that stop mid-value are half a tool call, and
    /// half a tool call spent as `exec {}` reaches the gates and runs.
    #[test]
    fn tool_arguments_cut_mid_value_are_refused_not_emptied() {
        let wire = json!({
            "status": "completed",
            "output": [ { "type": "function_call", "call_id": "fc_1",
                          "name": "exec", "arguments": "{\"cmd\": \"rm " } ],
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        });
        assert!(response_from(&wire).is_err());
    }
}
