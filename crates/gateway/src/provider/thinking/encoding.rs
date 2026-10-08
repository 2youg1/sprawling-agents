// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fields one thinking level, or thinking on, is written in on each
//! face (`crates/gateway/spec/Dialect.lean` §8-1, gateway D35).
//!
//! The one place a level becomes request bytes. The word for a level is
//! `Effort::as_str`; which field carries it is the face's and, on the
//! chat face, the host's (`EffortField`). Check the vendor's page before
//! changing a row: each `EffortField` variant names the documents it was
//! read from.

use kernel::DialectKind;
use serde_json::{Value, json};

use super::Ask;
use crate::provider::preset::EffortField;

/// The top-level fields one request carries for `ask` on this face.
/// Empty where the face has no field for it: the responses face and the
/// specification's own chat spelling have no switch, so thinking on is
/// written nowhere there and the provider's own default answers.
pub(crate) fn fields(ask: Ask, dialect: DialectKind, field: EffortField) -> Vec<(&'static str, Value)> {
    match dialect {
        // `effort` hangs under `output_config` and nowhere else; thinking
        // on is the adaptive mode the model list states.
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
