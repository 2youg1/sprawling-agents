// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an agent reports and asks during a turn, read off its
//! `session/update` and `session/request_permission` messages
//! (protocol-SPEC.md 8-19).
//!
//! These readings follow ACP's version 1 schema and nothing else: the
//! session decides when a message is read and where it goes, and this
//! module decides only what it says.

use serde_json::Value;

use super::text_at;

/// Something the agent reported during a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// A chunk of the agent's answer.
    Text(String),
    /// A chunk of the agent's reasoning.
    Thought(String),
    /// A tool call the agent started, in its own words.
    ToolCall {
        id: String,
        title: String,
        kind: String,
    },
    /// A tool call's status moved: `in_progress`, `completed`, `failed`.
    ToolCallStatus { id: String, status: String },
    /// Any other variant, by name: a plan, a usage count, a mode.
    Other { variant: String },
}

/// A permission the agent asked for before running a tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    pub title: String,
    pub options: Vec<PermitOption>,
}

/// One answer the agent offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitOption {
    pub id: String,
    pub name: String,
    pub kind: PermitKind,
}

/// What an offered answer would do, as the agent labels it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

/// One `session/update` payload as this client reads it. A variant it
/// has no reading for keeps its name, so nothing an agent reports is
/// dropped without a trace.
pub(super) fn update_of(update: &Value) -> Update {
    let variant = text_at(update, "sessionUpdate").unwrap_or_default();
    let chunk = || {
        update
            .get("content")
            .and_then(|content| text_at(content, "text"))
            .unwrap_or_default()
            .to_owned()
    };
    let field = |key: &str| text_at(update, key).unwrap_or_default().to_owned();
    match variant {
        "agent_message_chunk" => Update::Text(chunk()),
        "agent_thought_chunk" => Update::Thought(chunk()),
        "tool_call" => Update::ToolCall {
            id: field("toolCallId"),
            title: field("title"),
            kind: field("kind"),
        },
        "tool_call_update" => Update::ToolCallStatus {
            id: field("toolCallId"),
            status: field("status"),
        },
        other => Update::Other {
            variant: other.to_owned(),
        },
    }
}

/// One `session/request_permission` as this client reads it. An option
/// whose kind is not one of ACP's four is left out: offering a person a
/// choice whose effect nobody can name would be a guess.
pub(super) fn ask_of(params: &Value) -> PermissionAsk {
    let title = params
        .get("toolCall")
        .and_then(|call| text_at(call, "title"))
        .unwrap_or_default()
        .to_owned();
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let kind = match text_at(option, "kind")? {
                        "allow_once" => PermitKind::AllowOnce,
                        "allow_always" => PermitKind::AllowAlways,
                        "reject_once" => PermitKind::RejectOnce,
                        "reject_always" => PermitKind::RejectAlways,
                        _ => return None,
                    };
                    Some(PermitOption {
                        id: text_at(option, "optionId")?.to_owned(),
                        name: text_at(option, "name").unwrap_or_default().to_owned(),
                        kind,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    PermissionAsk { title, options }
}
