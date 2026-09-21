// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The turn typestate: Assembling -> Calling ->
//! ToolWave -> Recording, phase changes carrying `&mut dyn Ledger`.
//! Interrupts are consumed *only* at phase boundaries — the boundary
//! snapshot is a parameter of every transition, and no method exists that
//! could observe one mid-phase. That absence is A9's structural half.
//! A wave is many effects rather than one, so every call in it is a
//! boundary of the same kind: the executor is asked before each, and a
//! halt ends the wave there instead of after its last call.
//! Steer consumes at a boundary too, but advances: it is an addition to
//! the window, not an ending.
//!
//! All events of one turn share the timestamp given to [`Turn::begin`]:
//! order is `seq`'s business, time is a parameter, never sampled.

use kernel::event::record::{ModelCalled, ModelReturned, SteerReceived};
use kernel::model::content_from_message;
use kernel::{
    AxCode, AxError, B3Hash, BuildingPolicy, ChatRequest, ContentBlock, EventRef, Ledger, Model,
    ModelRequest, ModelReturn, ModelUsage, Payload, RunId, StopReason, TimeMs, ToolCall, ToolDef,
};

use crate::prefix::FrozenPrefix;
use crate::window::Window;

mod boundary;
mod ledger;
mod report;
mod wave;

use ledger::{Authored, Carried, Journal};

pub use boundary::{Interrupt, NextCall, PhaseOutcome, TurnCancelled};
pub use report::{CallShape, TurnReport};

/// The typestate carrier. Phase data lives in `S` and is private to this
/// module: a phase literal cannot be forged, a phase cannot be skipped,
/// and no method returns an earlier phase. Everything the turn writes
/// goes through `journal`, which is the module's only ledger door.
#[derive(Debug)]
pub struct Turn<S> {
    journal: Journal,
    state: S,
}

#[derive(Debug)]
pub struct Assembling(());

#[derive(Debug)]
pub struct Calling {
    segments: [B3Hash; 4],
    chat: ChatRequest,
}

#[derive(Debug)]
pub struct ToolWave {
    calls: Vec<ToolCall>,
    model_returned: EventRef,
    assistant: Vec<ContentBlock>,
    usage: Option<ModelUsage>,
    stop: Option<StopReason>,
}

#[derive(Debug)]
pub struct Recording {
    model_returned: EventRef,
    calls_made: usize,
    assistant: Vec<ContentBlock>,
    wave_results: Vec<ContentBlock>,
    usage: Option<ModelUsage>,
    stop: Option<StopReason>,
}

impl Turn<Assembling> {
    /// Opens a turn. `t` stamps every event of this turn; the executor
    /// advances it between turns (determinism rule 2).
    pub fn begin(run: RunId, who: String, t: TimeMs) -> Turn<Assembling> {
        Turn {
            journal: Journal::open(run, who, t),
            state: Assembling(()),
        }
    }

    /// Boundary 1 (before assembly). Builds the canonical request from
    /// the frozen prefix (system blocks), the window (messages) and the
    /// catalog's tool defs; appends `prompt_assembled` with the prefix's
    /// full source notes.
    pub fn assemble(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        prefix: &FrozenPrefix,
        window: &Window,
        tools: &[ToolDef],
        shape: &CallShape,
    ) -> Result<PhaseOutcome<Turn<Calling>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        // The per-turn digest, recomputed from the bytes this request
        // will carry and checked against what the session froze. It
        // comes before `prompt_assembled` is written: a refusal must not
        // leave a line describing a prefix the city will not send.
        let segments = prefix.verified_segment_hashes()?;
        let prompt = prefix.prompt_payload()?;
        self.journal
            .append_authored(ledger, Authored::PromptAssembled, prompt)?;
        let chat = ChatRequest {
            model: shape.model.clone(),
            max_tokens: shape.max_tokens,
            system: prefix.system_blocks()?,
            messages: window.messages().to_vec(),
            tools: tools.to_vec(),
            effort: shape.effort,
        };
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: Calling { segments, chat },
        }))
    }
}

impl Turn<Calling> {
    /// Boundary 2 (before the provider call). Appends `model_called` and
    /// `model_returned`; a provider Err propagates after nothing but the
    /// boundary consumption touched the ledger.
    /// `'sink` is named rather than elided because the caller holds the
    /// sink for the whole run and hands it to every turn: with an elided
    /// lifetime the reborrow would have to shrink the trait object's own
    /// lifetime, which `&mut` does not permit.
    pub fn call<'sink>(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        model: &mut dyn Model,
        policy: &BuildingPolicy,
        deltas: Option<&mut (dyn FnMut(&kernel::Increment) + 'sink)>,
    ) -> Result<PhaseOutcome<Turn<ToolWave>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let Calling { segments, chat } = self.state;
        // The hashes above describe the prefix; these describe the four
        // system blocks that will actually go on the wire. They are the
        // second per-turn digest, and they disagree only if the request
        // was built from something other than the prefix it names.
        let segments = crate::prefix::verified_system_hashes(&chat.system, &segments)?;
        let request = ModelRequest {
            policy: policy.clone(),
            segments,
            chat,
        };
        let called = ModelCalled {
            segments: request.segments.to_vec(),
            model: request.chat.model.clone(),
        };
        self.journal
            .append_authored(ledger, Authored::ModelCalled, Payload::of(&called)?)?;
        // The streaming door when somebody is watching, the blocking one
        // when nobody is. Both return the same `ModelReturn`, and the
        // record below is written from that return in either case - so
        // what a page sees arriving and what the ledger keeps cannot come
        // from two different readings of one reply.
        let returned_value = match deltas {
            Some(onto) => model.call_streaming(&request, onto)?,
            None => model.call(&request)?,
        };
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
        };
        let model_returned = self.journal.append_redacted(
            ledger,
            Carried::ModelReturned,
            Payload::of(&returned)?,
        )?;
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: ToolWave {
                calls,
                model_returned,
                assistant,
                usage,
                stop,
            },
        }))
    }
}

impl Turn<Recording> {
    /// Closes the turn. Recording is the accounting boundary: nothing
    /// extra is appended here, because every effect is already on the
    /// ledger. What the phase exists for is the fourth boundary — the
    /// last moment before the run acts on what this turn decided,
    /// including the work it handed down.
    ///
    /// # Errors
    /// Propagates the ledger's refusal to record the boundary event.
    pub fn record(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
    ) -> Result<PhaseOutcome<TurnReport>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        Ok(PhaseOutcome::Advanced(TurnReport {
            redacted: self.journal.redacted(),
            refs: self.journal.take_refs(),
            model_returned: self.state.model_returned,
            calls_made: self.state.calls_made,
            assistant: self.state.assistant,
            wave_results: self.state.wave_results,
            usage: self.state.usage,
            stop: self.state.stop,
        }))
    }
}

impl<S> Turn<S> {
    /// Ends the turn where it stands: `cancel_received` and the refs of
    /// everything this turn wrote.
    ///
    /// Reached from the boundary consumer below and from a wave that was
    /// halted between two calls, so both endings are one line written in
    /// one place.
    fn cancel_here(&mut self, ledger: &mut dyn Ledger) -> Result<TurnCancelled, AxError> {
        self.journal
            .append_authored(ledger, Authored::CancelReceived, Payload::empty())?;
        Ok(TurnCancelled {
            refs: self.journal.take_refs(),
        })
    }

    /// The one interrupt consumer. Cancel appends `cancel_received` and
    /// ends the turn; Steer appends `steer_received` and advances — the
    /// executor folds the text into its window for the next assembly.
    fn consume_boundary(
        &mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
    ) -> Result<Option<TurnCancelled>, AxError> {
        match interrupt {
            Interrupt::None => Ok(None),
            Interrupt::Cancel => Ok(Some(self.cancel_here(ledger)?)),
            Interrupt::Steer { source, text } => {
                let steer = SteerReceived { source, text };
                self.journal.append_authored(
                    ledger,
                    Authored::SteerReceived,
                    Payload::of(&steer)?,
                )?;
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
