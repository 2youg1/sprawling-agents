// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one call across the seam carries in each direction, and the two
//! conversions between assistant content and a ledger payload.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::wire::{
    BuildingPolicy, ChatRequest, ChatResponse, ContentBlock, ModelUsage, StopReason,
};
use crate::budget::UsdMicros;
use crate::error::{AxCode, AxError};
use crate::event::Payload;
use crate::locator::B3Hash;
use crate::tool::ToolCall;

/// The account-side form of assistant content: the single authority for
/// how content blocks enter `model_returned` payloads (and thus how the
/// window is rebuilt offline, C16).
pub fn message_payload(content: &[ContentBlock]) -> Result<Payload, AxError> {
    let blocks = serde_json::to_value(content).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "encode content blocks",
            err.to_string(),
        )
    })?;
    let mut map = Map::new();
    map.insert("content".to_owned(), blocks);
    Payload::new(map)
}

/// What the adapter needs to make one call. `segments` are the frozen
/// prefix hashes (same source as `prompt_assembled`); `chat` is the full
/// canonical conversation the dialect puts on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub policy: BuildingPolicy,
    pub segments: [B3Hash; 4],
    pub chat: ChatRequest,
}

/// One assistant turn. `message` is the in-window content; `calls` is the
/// requested tool wave — empty means the turn loop may conclude the run.
/// `usage`/`stop`/`billed_usd_micros` ride along from real dialects and
/// stay `None` for scripted adapters that predate cost accounting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelReturn {
    pub message: Payload,
    pub calls: Vec<ToolCall>,
    pub usage: Option<ModelUsage>,
    pub stop: Option<StopReason>,
    pub billed_usd_micros: Option<UsdMicros>,
}

impl ModelReturn {
    /// Scripted/minimal construction: content and wave only.
    pub fn bare(message: Payload, calls: Vec<ToolCall>) -> ModelReturn {
        ModelReturn {
            message,
            calls,
            usage: None,
            stop: None,
            billed_usd_micros: None,
        }
    }

    /// The one mapping from canonical response to seam return: tool_use
    /// blocks become the wave, everything is kept for the account.
    pub fn from_response(
        resp: ChatResponse,
        billed_usd_micros: Option<UsdMicros>,
    ) -> Result<ModelReturn, AxError> {
        let message = message_payload(&resp.content)?;
        let mut calls = Vec::new();
        for block in &resp.content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                calls.push(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    args: input.clone(),
                });
            }
        }
        Ok(ModelReturn {
            message,
            calls,
            usage: Some(resp.usage),
            stop: Some(resp.stop),
            billed_usd_micros,
        })
    }
}

/// The inverse of [`message_payload`] for window folding and rebuild.
///
/// A message without a canonical `content` array folds to no blocks —
/// defined behavior for scripted payloads, not an error path. A `content`
/// array that is present but unreadable is an error: silently folding it
/// to nothing would drop an assistant message the ledger still holds, and
/// a window that disagrees with the ledger is a second history.
pub fn content_from_message(message: &Payload) -> Result<Vec<ContentBlock>, AxError> {
    let Some(value) = message.as_map().get("content") else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value.clone()).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "read assistant content",
            err.to_string(),
        )
        .with_recovery(
            "this build cannot read a block kind the ledger holds; replay with the build that \
             wrote it, or extend ContentBlock",
        )
    })
}

/// Guard for wire faces: `serde_json::Value` trees entering payload-adjacent
/// positions must respect the float ban before conversion.
pub fn value_has_float(value: &Value) -> bool {
    match value {
        Value::Number(n) => !n.is_i64() && !n.is_u64(),
        Value::Array(items) => items.iter().any(value_has_float),
        Value::Object(map) => map.values().any(value_has_float),
        Value::Null | Value::Bool(_) | Value::String(_) => false,
    }
}
