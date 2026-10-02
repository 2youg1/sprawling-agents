// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face of `describe` (`crates/runtime/Spec.lean` §8-60): what an
//! admitted capability is, read from the run's catalog. Its answer is an
//! ordinary tool result appended at the end of the conversation, so the
//! request's tool list and its prompt cache do not move (runtime D21).

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::Value;

use crate::catalog::Catalog;

const ACTION: &str = "describe";

/// The tool itself; it holds the catalog rather than a copy, as `read`
/// does, so what it answers is what the run was admitted.
pub struct DescribeTool {
    catalog: Arc<Mutex<Catalog>>,
    meta: ToolMeta,
}

impl DescribeTool {
    /// The name this tool answers to, read by its registration and by
    /// `mode::core_tools`.
    pub const NAME: &'static str = "describe";

    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(catalog: Arc<Mutex<Catalog>>) -> Result<DescribeTool, AxError> {
        Ok(DescribeTool {
            catalog,
            meta: ToolMeta {
                name: ToolName::parse(Self::NAME)?,
                disclosure: "Get the whole guide of a capability this building admits: a tool's \
                             description and input schema, or a skill's line. Give its whole \
                             name, or a few words to get the closest names."
                    .to_owned(),
                params: Payload::of(&serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "a name from the dormant \
                            index, or a few words of what you need" },
                    },
                    "required": ["name"],
                }))?,
                effect: Effect::Read,
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }
}

impl Tool for DescribeTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                Self::NAME
            )));
        }
        let asked = call
            .args
            .as_map()
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    ACTION,
                    "missing string argument `name`",
                )
                .with_recovery("pass `name`: a capability's name, or a few words")
            })?;
        let guide = self
            .catalog
            .lock()
            .map_err(|_| poisoned())?
            .describe(asked)?;
        Ok(ToolOutcome {
            result: Payload::of(&serde_json::json!({ "text": guide }))?,
            attachments: Vec::new(),
        })
    }
}

/// The catalog's lock was left by a thread that died; the same answer
/// `read` gives (`crates/runtime/spec/Tools/Read.lean`).
pub(super) fn poisoned() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        ACTION,
        "the catalog was left locked by a thread that died",
    )
    .with_recovery(
        "end this run and resume it: the catalog cannot be trusted again inside a process \
         where a thread died holding it",
    )
}
