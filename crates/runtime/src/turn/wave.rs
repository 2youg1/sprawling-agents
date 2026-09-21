// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, executed in call order.

use kernel::{AxCode, AxError, ContentBlock, Ledger, ToolCall, ToolOutcome};
use serde_json::{Map, Value};

use super::{Carried, Interrupt, NextCall, PhaseOutcome, Recording, ToolWave, Turn};

impl Turn<ToolWave> {
    /// Boundary 3 (before tool execution). Serial: parallel execution
    /// with serial accounting stays future work; accounting order is the
    /// call order. A tool Err is not a turn Err — it lands in
    /// `tool_result` and goes back to the model (the model is the
    /// recovery subject).
    ///
    /// Both tool events reach the ledger through `append_redacted`: a
    /// tool's arguments and its result are the two payloads most likely
    /// to quote a credential the work just read.
    ///
    /// `still_going` is asked before every call, with that call's index.
    /// One question per wave left a halted scope running whatever the
    /// model asked for in one reply: eight edits are eight effects, and
    /// the person who stopped the city waited for all of them.
    pub fn execute(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        invoke: &mut dyn FnMut(&ToolCall) -> Result<ToolOutcome, AxError>,
        still_going: &mut dyn FnMut(u32) -> NextCall,
    ) -> Result<PhaseOutcome<Turn<Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls = std::mem::take(&mut self.state.calls);
        let mut wave_results = Vec::new();
        let mut index = 0u32;
        for call in &calls {
            // Asked before the call is written down, so a wave that was
            // stopped leaves no `tool_called` line for work nothing ever
            // did.
            let standing = still_going(index);
            index = index.saturating_add(1);
            match standing {
                NextCall::Allowed => {}
                // Through the same door a phase boundary takes, so a
                // wave that stops leaves the one `cancel_received` line
                // every other ending leaves.
                NextCall::Halted => {
                    return Ok(PhaseOutcome::Cancelled(self.cancel_here(ledger)?));
                }
            }
            let mut called = Map::new();
            called.insert("id".to_owned(), Value::String(call.id.clone()));
            called.insert("name".to_owned(), Value::String(call.name.to_string()));
            called.insert(
                "args".to_owned(),
                serde_json::to_value(&call.args).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode tool args", err.to_string())
                        .with_recovery(
                            "call the tool again with arguments made of strings and \
                             whole numbers, which is all this city's payloads carry",
                        )
                })?,
            );
            self.journal
                .append_redacted(ledger, Carried::ToolCalled, called)?;
            let mut result = Map::new();
            result.insert("tool_use_id".to_owned(), Value::String(call.id.clone()));
            result.insert("name".to_owned(), Value::String(call.name.to_string()));
            let mut pictures = Vec::new();
            let (content, is_error) = match invoke(call) {
                Ok(ToolOutcome {
                    result: outcome,
                    attachments,
                }) => {
                    pictures = attachments;
                    let value = serde_json::to_value(&outcome).map_err(|err| {
                        AxError::failure(AxCode::InvalidArgs, "encode tool result", err.to_string())
                            .with_recovery(
                                "report this against the tool that answered: a result \
                                 payload holds strings and whole numbers only",
                            )
                    })?;
                    result.insert("result".to_owned(), value.clone());
                    (
                        serde_json::to_string(&value).map_err(|err| {
                            AxError::failure(
                                AxCode::InvalidArgs,
                                "encode tool result",
                                err.to_string(),
                            )
                            .with_recovery(
                                "report this against runtime::turn::wave: the value \
                                 printed here was accepted as JSON one line above",
                            )
                        })?,
                        false,
                    )
                }
                Err(tool_err) => {
                    let value = serde_json::to_value(&tool_err).map_err(|err| {
                        AxError::failure(AxCode::InvalidArgs, "encode tool error", err.to_string())
                            .with_recovery(
                                "report this against runtime::turn::wave: an AxError \
                                 is seven fields of text, numbers and booleans",
                            )
                    })?;
                    result.insert("error".to_owned(), value.clone());
                    (
                        serde_json::to_string(&value).map_err(|err| {
                            AxError::failure(
                                AxCode::InvalidArgs,
                                "encode tool error",
                                err.to_string(),
                            )
                            .with_recovery(
                                "report this against runtime::turn::wave: the value \
                                 printed here was accepted as JSON one line above",
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
                // What the tool produced, carried through unchanged: the
                // bytes are already in the content store, so this is a
                // reference and four integers whatever the picture is.
                attachments: pictures,
            });
            self.journal
                .append_redacted(ledger, Carried::ToolResult, result)?;
        }
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: Recording {
                model_returned: self.state.model_returned,
                calls_made: calls.len(),
                assistant: self.state.assistant,
                wave_results,
                usage: self.state.usage,
                stop: self.state.stop,
            },
        }))
    }
}
