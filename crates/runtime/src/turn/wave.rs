// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, executed in call order.

use kernel::{AxCode, AxError, ContentBlock, EventKind, Ledger, ToolCall, ToolOutcome};
use serde_json::{Map, Value};

use super::{Interrupt, PhaseOutcome, Recording, ToolWave, Turn, payload};

impl Turn<ToolWave> {
    /// Boundary 3 (before tool execution). Serial: parallel execution
    /// with serial accounting stays future work; accounting order is the
    /// call order. A tool Err is not a turn Err — it lands in
    /// `tool_result` and goes back to the model (the model is the
    /// recovery subject).
    pub fn execute(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        invoke: &mut dyn FnMut(&ToolCall) -> Result<ToolOutcome, AxError>,
    ) -> Result<PhaseOutcome<Turn<Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls = std::mem::take(&mut self.state.calls);
        let mut wave_results = Vec::new();
        for call in &calls {
            let mut called = Map::new();
            called.insert("id".to_owned(), Value::String(call.id.clone()));
            called.insert("name".to_owned(), Value::String(call.name.to_string()));
            called.insert(
                "args".to_owned(),
                serde_json::to_value(&call.args).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode tool args", err.to_string())
                })?,
            );
            let echo = ledger.append(self.draft(EventKind::ToolCalled, payload(called)?))?;
            self.refs.push(echo);
            let mut result = Map::new();
            result.insert("tool_use_id".to_owned(), Value::String(call.id.clone()));
            result.insert("name".to_owned(), Value::String(call.name.to_string()));
            let (content, is_error) = match invoke(call) {
                Ok(ToolOutcome { result: outcome }) => {
                    let value = serde_json::to_value(&outcome).map_err(|err| {
                        AxError::failure(AxCode::InvalidArgs, "encode tool result", err.to_string())
                    })?;
                    result.insert("result".to_owned(), value.clone());
                    (
                        serde_json::to_string(&value).map_err(|err| {
                            AxError::failure(
                                AxCode::InvalidArgs,
                                "encode tool result",
                                err.to_string(),
                            )
                        })?,
                        false,
                    )
                }
                Err(tool_err) => {
                    let value = serde_json::to_value(&tool_err).map_err(|err| {
                        AxError::failure(AxCode::InvalidArgs, "encode tool error", err.to_string())
                    })?;
                    result.insert("error".to_owned(), value.clone());
                    (
                        serde_json::to_string(&value).map_err(|err| {
                            AxError::failure(
                                AxCode::InvalidArgs,
                                "encode tool error",
                                err.to_string(),
                            )
                        })?,
                        true,
                    )
                }
            };
            wave_results.push(ContentBlock::ToolResult {
                tool_use_id: call.id.clone(),
                content,
                is_error,
                // Tools state their outcome in text today; a tool that
                // produces a picture fills this in where it runs.
                attachments: Vec::new(),
            });
            let echo = ledger.append(self.draft(EventKind::ToolResult, payload(result)?))?;
            self.refs.push(echo);
        }
        Ok(PhaseOutcome::Advanced(Turn {
            run: self.run,
            who: self.who,
            t: self.t,
            refs: self.refs,
            state: Recording {
                model_returned: self.state.model_returned,
                calls_made: calls.len(),
                assistant: self.state.assistant,
                wave_results,
                usage: self.state.usage,
            },
        }))
    }
}
