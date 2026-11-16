// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::event::payload::Payload;
use crate::tool::{Effect, RenderIntent, ToolName};

const SUBJECT_KEYS: [&str; 4] = ["file_path", "path", "addr", "command"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolCalled {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    pub args: Payload,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<RenderIntent>,
}

impl ToolCalled {
    #[must_use]
    pub fn subject_of(args: &Payload) -> Option<String> {
        for key in SUBJECT_KEYS {
            if let Some(named) = args.as_map().get(key).and_then(serde_json::Value::as_str) {
                return Some(named.to_owned());
            }
        }
        args.as_map()
            .values()
            .find_map(serde_json::Value::as_str)
            .map(str::to_owned)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolResult {
    pub tool_use_id: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub name: ToolName,
    #[serde(flatten)]
    pub answer: ToolAnswer,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_us: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered: Option<Rendering>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown_bytes: Option<u64>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<crate::ImageRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Rendering {
    V1,

    V2,

    V3,

    V4,
}

impl Rendering {
    #[must_use]
    pub fn of(recorded: Option<Rendering>) -> Rendering {
        recorded.unwrap_or(Rendering::V1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ToolAnswer {
    Answered { result: Payload },

    Failed { error: Payload },
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
