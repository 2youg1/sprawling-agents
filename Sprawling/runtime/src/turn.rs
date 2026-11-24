// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;

use kernel::event::record::{CancelReceived, ModelReturned, SteerReceived};
use kernel::model::content_from_message;
use kernel::{
    Address, AxCode, AxError, B3Hash, BuildingPolicy, ChatRequest, ContentBlock, Ledger, Model,
    ModelRequest, ModelReturn, ModelUsage, Payload, StopReason, TimeMs, ToolCall, ToolDef,
};

use crate::compaction::Exchange;
use crate::conversation::Conversation;

mod boundary;
mod ledger;
mod prompt;
mod recovery;
mod report;
mod speculation;
mod wave;

pub(crate) use ledger::RunLine;
use ledger::{Authored, Carried, Journal};
pub use ledger::{Entry, HeldLines};

pub use boundary::{Interrupt, NextCall, PhaseOutcome, TurnCancelled};
pub use prompt::{PromptRecord, RunPrompt};
pub use report::{CallShape, TurnReport};
pub use speculation::Generating;
pub use wave::{Admitted, ConcurrentInvoke};

#[derive(Debug)]
pub struct Turn<'h, S> {
    journal: Journal<'h>,
    state: S,
}

#[derive(Debug)]
pub struct Assembling(());

#[derive(Debug)]
pub struct Calling<'c> {
    segments: [B3Hash; 4],
    chat: ChatRequest<'c>,

    round: Option<&'c mut crate::Watchdog>,
}

#[derive(Debug)]
pub struct ToolWave {
    calls: Vec<ToolCall>,
    speculated: speculation::Speculated,
    model_returned: Entry,
    assistant: Vec<ContentBlock>,
    usage: Option<ModelUsage>,
    stop: Option<StopReason>,
}

#[derive(Debug)]
pub struct Recording {
    model_returned: Entry,
    calls_made: usize,
    exchange: Exchange,
    usage: Option<ModelUsage>,
    stop: Option<StopReason>,
}

impl<'h> Turn<'h, Assembling> {
    pub fn begin(
        lines: &'h mut HeldLines,
        t: TimeMs,
        now: &'h mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Turn<'h, Assembling> {
        Turn {
            journal: Journal::open(lines, t, now),
            state: Assembling(()),
        }
    }

    #[must_use]
    pub fn timed(mut self, monotonic_us: &'h mut dyn FnMut() -> u64) -> Turn<'h, Assembling> {
        self.journal.time_with(monotonic_us);
        self
    }

    pub fn assemble<'c>(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        prompt: RunPrompt<'_>,
        conversation: &'c Conversation,
        tools: &'c [ToolDef],
        shape: &CallShape,
    ) -> Result<PhaseOutcome<Turn<'h, Calling<'c>>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }

        let segments = prompt.prefix.verified_segment_hashes()?;

        let plan = crate::prefix::BreakpointPlan::for_conversation(conversation.messages());
        let payload = prompt.prefix.prompt_payload(&plan)?;
        if !prompt.recorded.holds(&payload) {
            self.journal
                .append_authored(Authored::PromptAssembled, payload.clone());
            prompt.recorded.remember(payload);
        }
        let chat = ChatRequest {
            model: shape.model.clone(),
            max_tokens: shape.generation.max_tokens,
            system: prompt.prefix.system_blocks()?,
            messages: Cow::Borrowed(conversation.messages()),
            tools: Cow::Borrowed(tools),
            breakpoint: plan.message_breakpoint(),
            shared: prompt.prefix.shared_part(),
            effort: shape.effort,
            generation: shape.generation.settings.clone(),
        };
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: Calling {
                segments,
                chat,
                round: None,
            },
        }))
    }
}

impl<'h, 'c> Turn<'h, Calling<'c>> {
    #[must_use]
    pub fn under(self, watchdog: &'c mut crate::Watchdog) -> Turn<'h, Calling<'c>> {
        Turn {
            journal: self.journal,
            state: Calling {
                round: Some(watchdog),
                ..self.state
            },
        }
    }
}

impl<'h> Turn<'h, Calling<'_>> {
    pub fn call<'sink>(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        model: &mut dyn Model,
        policy: &BuildingPolicy,
        generating: Generating<'_, 'sink>,
    ) -> Result<PhaseOutcome<Turn<'h, ToolWave>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let Calling {
            segments,
            chat,
            round,
        } = self.state;

        let segments = crate::prefix::verified_system_hashes(&chat.system, &segments)?;
        let request = ModelRequest {
            policy: policy.clone(),
            segments,
            chat,
        };

        let mut call = recovery::ModelCall::open(&mut self.journal, ledger, model, &request);

        let mut unheld = crate::Watchdog::default();
        let mut repair = recovery::BlockingResend {
            round: round.unwrap_or(&mut unheld),
        };
        let recovery::Settled {
            returned: returned_value,
            speculated,
            first_at,
            first_us,
            sent_us,
        } = call.ask(&mut [&mut repair], generating)?;
        let arrived = self.journal.read_moment()?;
        let ModelReturn {
            message,
            calls,
            usage,
            stop,
            billed_usd_micros,
        } = returned_value;
        let assistant = content_from_message(&message)?;
        let calls_len = u64::try_from(calls.len()).map_err(|_| {
            AxError::failure(AxCode::InvalidArgs, "encode model return", "wave too large")
                .with_recovery(
                    "lower this model's max output tokens so it asks for fewer tool \
                     calls in one reply",
                )
        })?;
        let returned = ModelReturned {
            message,
            calls: calls_len,
            usage,
            stop,
            billed_usd_micros,
            first_at,
            first_us,
            took_us: arrived.since(sent_us),
        };
        let model_returned = self.journal.append_redacted(
            Carried::ModelReturned { at: arrived.at },
            Payload::of(&returned)?,
        )?;
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: ToolWave {
                calls,
                speculated,
                model_returned,
                assistant,
                usage,
                stop,
            },
        }))
    }
}

impl Turn<'_, Recording> {
    pub fn record(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
    ) -> Result<PhaseOutcome<TurnReport>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let Recording {
            model_returned,
            calls_made,
            mut exchange,
            usage,
            stop,
        } = self.state;
        let shortened = exchange.compact()?;
        Ok(PhaseOutcome::Advanced(TurnReport {
            redacted: self.journal.redacted(),
            shortened,
            model_returned,
            calls_made,
            assistant: exchange.assistant().to_vec(),
            wave_results: exchange.results().to_vec(),
            usage,
            stop,
        }))
    }
}

impl<S> Turn<'_, S> {
    pub(crate) fn hold_run_line(&mut self, line: RunLine, addr: Address, data: Payload) {
        self.journal.append_run_line(line, addr, data);
    }

    fn cancel_here(&mut self, ledger: &mut dyn Ledger) -> Result<TurnCancelled, AxError> {
        self.journal
            .append_authored(Authored::CancelReceived, Payload::of(&CancelReceived {})?);
        let lines = self.journal.lines();
        lines.barrier(ledger)?;
        Ok(TurnCancelled {
            refs: lines.refs().to_vec(),
        })
    }

    fn consume_boundary(
        &mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
    ) -> Result<Option<TurnCancelled>, AxError> {
        match interrupt {
            Interrupt::None | Interrupt::Policy { .. } => Ok(None),
            Interrupt::Cancel => Ok(Some(self.cancel_here(ledger)?)),
            Interrupt::Steer {
                speaker,
                text,
                pictures,
                request,
            } => {
                let steer = SteerReceived {
                    pictures: pictures.clone(),
                    source: speaker.recorded(),
                    text,
                    idem: request,
                };
                self.journal
                    .append_authored(Authored::SteerReceived, Payload::of(&steer)?);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
