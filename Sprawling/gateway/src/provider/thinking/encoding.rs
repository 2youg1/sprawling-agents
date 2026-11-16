// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::DialectKind;
use serde_json::{Value, json};

use super::Ask;
use crate::provider::preset::EffortField;

pub(crate) fn fields(
    ask: Ask,
    dialect: DialectKind,
    field: EffortField,
) -> Vec<(&'static str, Value)> {
    match dialect {
        DialectKind::Anthropic => match ask {
            Ask::Level(level) => vec![("output_config", json!({ "effort": level.as_str() }))],
            Ask::On => vec![("thinking", json!({ "type": "adaptive" }))],
        },
        DialectKind::OpenAiResponses => match ask {
            Ask::Level(level) => vec![("reasoning", json!({ "effort": level.as_str() }))],
            Ask::On => Vec::new(),
        },
        DialectKind::OpenAi => chat(ask, field),
    }
}

fn chat(ask: Ask, field: EffortField) -> Vec<(&'static str, Value)> {
    let level = |level: kernel::Effort| ("reasoning_effort", json!(level.as_str()));
    match (field, ask) {
        (EffortField::ReasoningEffort, Ask::Level(chosen)) => vec![level(chosen)],
        (EffortField::ReasoningEffort, Ask::On) => Vec::new(),
        (EffortField::ReasoningObject, Ask::Level(chosen)) => {
            vec![("reasoning", json!({ "effort": chosen.as_str() }))]
        }
        (EffortField::ReasoningObject, Ask::On) => vec![("reasoning", json!({ "enabled": true }))],
        (EffortField::ThinkingToggle, Ask::Level(chosen)) => {
            vec![("thinking", json!({ "type": "enabled" })), level(chosen)]
        }
        (EffortField::ThinkingToggle, Ask::On) => vec![("thinking", json!({ "type": "enabled" }))],
        (EffortField::EnableThinking, Ask::Level(chosen)) => {
            vec![("enable_thinking", Value::Bool(true)), level(chosen)]
        }
        (EffortField::EnableThinking, Ask::On) => vec![("enable_thinking", Value::Bool(true))],
    }
}
