// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::usage::ModelUsage;
use super::wire::{BuildingPolicy, ChatRequest, ChatResponse, ContentBlock, StopReason};
use crate::budget::UsdMicros;
use crate::error::{AxCode, AxError};
use crate::event::Payload;
use crate::locator::B3Hash;
use crate::tool::ToolCall;

pub fn message_payload(content: &[ContentBlock]) -> Result<Payload, AxError> {
    let blocks = serde_json::to_value(content).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "encode content blocks",
            format!("{} block(s) of assistant content", content.len()),
        )
        .with_recovery(format!(
            "JSON refused these blocks ({err}); ask the provider dialect to return text, \
             tool calls, and thinking blocks only"
        ))
    })?;
    let mut map = Map::new();
    map.insert("content".to_owned(), blocks);
    Payload::new(map)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest<'a> {
    pub policy: BuildingPolicy,
    pub segments: [B3Hash; 4],
    pub chat: ChatRequest<'a>,
}

impl ModelRequest<'_> {

    pub fn into_owned(self) -> ModelRequest<'static> {
        ModelRequest {
            policy: self.policy,
            segments: self.segments,
            chat: self.chat.into_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelReturn {
    pub message: Payload,
    pub calls: Vec<ToolCall>,
    pub usage: Option<ModelUsage>,
    pub stop: Option<StopReason>,
    pub billed_usd_micros: Option<UsdMicros>,
}

impl ModelReturn {

    pub fn bare(message: Payload, calls: Vec<ToolCall>) -> ModelReturn {
        ModelReturn {
            message,
            calls,
            usage: None,
            stop: None,
            billed_usd_micros: None,
        }
    }

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
