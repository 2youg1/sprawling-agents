// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Scripted tools: the second adapter of the tool seam. Every failure
//! mode is injectable — an outcome script may hold typed errors, and an
//! exhausted script is itself a failure (E_TOOL_UNAVAILABLE).
//!
//! Specified by `tools/citysim/spec/Executor.lean` §8-2.

use std::collections::VecDeque;
use std::sync::Mutex;

use kernel::{AxCode, AxError, Tool, ToolCall, ToolMeta, ToolOutcome};

pub struct ScriptTool {
    meta: ToolMeta,
    outcomes: Mutex<VecDeque<Result<ToolOutcome, AxError>>>,
}

impl ScriptTool {
    pub fn new(meta: ToolMeta, outcomes: Vec<Result<ToolOutcome, AxError>>) -> Self {
        ScriptTool {
            meta,
            outcomes: Mutex::new(outcomes.into()),
        }
    }
}

impl Tool for ScriptTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "invoke scripted tool",
                call.name.to_string(),
            )
            .with_nearby(vec![self.meta.name.to_string()])
            .with_recovery("call this tool by its registered name"));
        }
        let next = self
            .outcomes
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "invoke scripted tool",
                    call.name.to_string(),
                )
                .with_recovery(
                    "an earlier scripted call died holding the script; rerun the scenario",
                )
            })?
            .pop_front();
        next.unwrap_or_else(|| {
            Err(AxError::failure(
                AxCode::ToolUnavailable,
                "invoke scripted tool",
                call.name.to_string(),
            )
            .with_recovery("the outcome script is exhausted; extend the scenario"))
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::{CostTier, Effect, Payload, RenderIntent, Temporal, ToolName};

    fn meta(name: &str) -> ToolMeta {
        ToolMeta {
            name: ToolName::parse(name).unwrap(),
            disclosure: "scripted probe; call it when the scenario says so".into(),
            params: Payload::empty(),
            effect: Effect::Read,
            cost_tier: CostTier::Free,
            timeout: None,
            render: RenderIntent::Generic,
            temporal: Temporal::Timeless,
        }
    }

    fn call(name: &str) -> ToolCall {
        ToolCall {
            id: "call-t".to_owned(),
            name: ToolName::parse(name).unwrap(),
            args: Payload::empty(),
        }
    }

    #[test]
    fn a_call_by_another_name_is_refused_with_its_own_name_nearby() {
        let tool = ScriptTool::new(meta("probe"), vec![]);
        let err = tool.invoke(&call("ghost")).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert_eq!(err.nearby(), ["probe"]);
    }

    #[test]
    fn exhausted_script_is_a_typed_failure() {
        let tool = ScriptTool::new(meta("probe"), vec![]);
        let err = tool.invoke(&call("probe")).unwrap_err();
        assert_eq!(err.code(), &AxCode::ToolUnavailable);
    }

    #[test]
    fn passes_the_tool_conformance_suite() {
        let mut tool = ScriptTool::new(
            meta("probe"),
            vec![Ok(ToolOutcome {
                result: Payload::empty(),
                attachments: Vec::new(),
            })],
        );
        kernel::tool::conformance::assert_tool_conformance(&mut tool);
    }
}
