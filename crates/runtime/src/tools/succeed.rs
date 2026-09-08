// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face succession shows a model: ask to be replaced by a successor
//! at the same address, the same depth, and therefore with the same
//! tools — and the desk that holds the ask until the run freezes.
//!
//! **Succession is not delegation.** `delegate` adds one to the depth
//! and hands part of the work to somebody else; `succeed` hands all of
//! it to the next run of the same resident, which is why a successor
//! may still delegate. No person is asked: nothing outside the run's
//! own room changes, no depth is added, and what bounds a chain of
//! successions is `Halt`, the same brake that bounds everything else.
//!
//! **A request is not a run.** The tool answers with where the successor
//! will start, not with a result, for the reason `delegate` gives: a run
//! is built by the assembly layer, and a tool that drove one would be
//! driving a run from inside another run's tool bench. The desk is read
//! when the run concludes; a cancelled run is not succeeded.

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

/// What a run said when it asked to be replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Succession {
    pub reason: String,
}

/// Holds at most one ask. A second ask replaces the first, reason
/// included: the run is replaced once whatever it says, and the last
/// reason it gave is the one worth keeping.
#[derive(Debug, Default)]
pub struct SuccessionDesk {
    asked: Option<Succession>,
}

impl SuccessionDesk {
    #[must_use]
    pub fn new() -> SuccessionDesk {
        SuccessionDesk::default()
    }

    pub fn ask(&mut self, succession: Succession) {
        self.asked = Some(succession);
    }

    #[must_use]
    pub fn asked(&self) -> Option<&Succession> {
        self.asked.as_ref()
    }

    /// Takes the ask, leaving the desk empty. Called once when the run
    /// concludes.
    pub fn take(&mut self) -> Option<Succession> {
        self.asked.take()
    }
}

/// The tool itself.
pub struct SucceedTool {
    desk: std::sync::Arc<std::sync::Mutex<SuccessionDesk>>,
    meta: ToolMeta,
}

impl SucceedTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(
        desk: std::sync::Arc<std::sync::Mutex<SuccessionDesk>>,
    ) -> Result<SucceedTool, AxError> {
        let mut reason = Map::new();
        reason.insert("type".to_owned(), Value::String("string".to_owned()));
        reason.insert(
            "description".to_owned(),
            Value::String("one line: why this run hands over now".to_owned()),
        );
        let mut properties = Map::new();
        properties.insert("reason".to_owned(), Value::Object(reason));
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("reason".to_owned())]),
        );
        Ok(SucceedTool {
            desk,
            meta: ToolMeta {
                name: ToolName::parse("succeed")?,
                disclosure: "Replace yourself with a successor at this address when the \
                             window fills: same tools, same depth, no person asked. Write \
                             Handoff.md in your room first; the successor starts from it and \
                             from your transcript."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Read,
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }
}

impl Tool for SucceedTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "ask for a successor",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let reason = call
            .args
            .as_map()
            .get("reason")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "ask for a successor",
                    "missing string argument `reason`",
                )
                .with_recovery("say in one line why this run hands over now")
            })?;
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "ask for a successor",
                "the desk was left locked by a thread that died",
            )
        })?;
        desk.ask(Succession {
            reason: reason.to_owned(),
        });
        let mut out = Map::new();
        out.insert(
            "starts".to_owned(),
            Value::String("when this run freezes, at this address".to_owned()),
        );
        out.insert(
            "before_then".to_owned(),
            Value::String(
                "write Handoff.md in your room, then end your turn without calling a tool"
                    .to_owned(),
            ),
        );
        Ok(ToolOutcome {
            result: Payload::new(out)?,
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

    fn call(args: Value) -> ToolCall {
        ToolCall {
            id: "tu_1".to_owned(),
            name: ToolName::parse("succeed").unwrap(),
            args: Payload::new(args.as_object().cloned().unwrap()).unwrap(),
        }
    }

    #[test]
    fn asking_lands_on_the_desk_and_the_last_reason_wins() {
        let desk = std::sync::Arc::new(std::sync::Mutex::new(SuccessionDesk::new()));
        let mut tool = SucceedTool::new(std::sync::Arc::clone(&desk)).unwrap();
        tool.invoke(&call(serde_json::json!({ "reason": "first" })))
            .unwrap();
        tool.invoke(&call(serde_json::json!({ "reason": "second" })))
            .unwrap();
        assert_eq!(desk.lock().unwrap().asked().unwrap().reason, "second");
        let taken = desk.lock().unwrap().take().unwrap();
        assert_eq!(taken.reason, "second");
        assert!(
            desk.lock().unwrap().asked().is_none(),
            "taken once, empty after"
        );
    }

    #[test]
    fn a_missing_reason_is_refused_and_nothing_lands() {
        let desk = std::sync::Arc::new(std::sync::Mutex::new(SuccessionDesk::new()));
        let mut tool = SucceedTool::new(std::sync::Arc::clone(&desk)).unwrap();
        let err = tool.invoke(&call(serde_json::json!({}))).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(desk.lock().unwrap().asked().is_none());
    }

    #[test]
    fn succession_asks_no_person_and_adds_no_depth() {
        let desk = std::sync::Arc::new(std::sync::Mutex::new(SuccessionDesk::new()));
        let tool = SucceedTool::new(desk).unwrap();
        assert_eq!(
            tool.meta().effect,
            Effect::Read,
            "not a spawn: no depth, no gate"
        );
    }
}
