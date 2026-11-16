// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use super::text_at;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {

    Text(String),

    Thought(String),

    ToolCall {
        id: String,
        title: String,
        kind: String,
    },

    ToolCallStatus { id: String, status: String },

    Other { variant: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    pub title: String,
    pub options: Vec<PermitOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitOption {
    pub id: String,
    pub name: String,
    pub kind: PermitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

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
