// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face of `call` (`crates/runtime/Spec.lean` §8-61). A call through
//! it is resolved at the tool face's entrance into the call it stands for,
//! which then passes its own tool's doors; this tool is reached only when
//! that resolution refused. It asks the catalog the same question again
//! and answers its refusal, so the refusal has one author.

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};

use crate::catalog::Catalog;

/// The tool itself.
pub struct CallTool {
    catalog: Arc<Mutex<Catalog>>,
    meta: ToolMeta,
}

impl CallTool {
    /// The name this tool answers to, read by its registration, by
    /// `mode::core_tools` and by the catalog's resolution.
    pub const NAME: &'static str = "call";

    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(catalog: Arc<Mutex<Catalog>>) -> Result<CallTool, AxError> {
        Ok(CallTool {
            catalog,
            meta: ToolMeta {
                name: ToolName::parse(Self::NAME)?,
                disclosure: "Run a tool from the dormant index by its name. `describe` it first \
                             for its input schema; arguments that do not fit are refused with \
                             the schema."
                    .to_owned(),
                params: Payload::of(&serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "the tool to run, as the \
                            dormant index or `describe` spells it" },
                        "args": { "type": "object", "description": "the tool's own \
                            arguments, as its input schema asks" },
                    },
                    "required": ["name", "args"],
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

impl Tool for CallTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "call a dormant tool",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                Self::NAME
            )));
        }
        let resolved = self
            .catalog
            .lock()
            .map_err(|_| super::describe::poisoned())?
            .resolve_call(call)?;
        // Resolution succeeds here only when the tool face that drove this
        // call skipped it; running the tool from here would pass none of
        // its doors.
        Err(AxError::failure(
            AxCode::ToolUnavailable,
            "call a dormant tool",
            format!("`{}` was not resolved before it ran", resolved.name),
        )
        .with_recovery(
            "report this against the tool face driving this run: it must resolve a `call` \
             before admitting it",
        ))
    }
}
