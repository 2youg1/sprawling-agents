// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, executed in call order.

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{AxCode, AxError, ContentBlock, Ledger, Payload, ToolCall, ToolOutcome};

use super::{Carried, Interrupt, NextCall, PhaseOutcome, Recording, ToolWave, Turn};

/// What the model reads back as the text of a tool result: the same
/// payload the ledger keeps, printed once so the two cannot disagree.
fn printed(payload: &Payload, action: &'static str) -> Result<String, AxError> {
    serde_json::to_string(payload).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, action, err.to_string()).with_recovery(
            "report this against runtime::turn::wave: a payload refused JSON after \
             its construction already accepted it",
        )
    })
}

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
            let called = ToolCalled {
                id: call.id.clone(),
                name: call.name.clone(),
                args: call.args.clone(),
            };
            self.journal
                .append_redacted(ledger, Carried::ToolCalled, Payload::of(&called)?)?;
            let mut pictures = Vec::new();
            let (answer, content, is_error) = match invoke(call) {
                Ok(ToolOutcome {
                    result: outcome,
                    attachments,
                }) => {
                    pictures = attachments;
                    (
                        ToolAnswer::Answered {
                            result: outcome.clone(),
                        },
                        printed(&outcome, "encode tool result")?,
                        false,
                    )
                }
                Err(tool_err) => {
                    let error = Payload::of(&tool_err)?;
                    (
                        ToolAnswer::Failed {
                            error: error.clone(),
                        },
                        printed(&error, "encode tool error")?,
                        true,
                    )
                }
            };
            let result = ToolResult {
                tool_use_id: call.id.clone(),
                name: call.name.clone(),
                answer,
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
                .append_redacted(ledger, Carried::ToolResult, Payload::of(&result)?)?;
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
