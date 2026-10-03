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
//! Lines that record a moment the turn waited for - `model_called`,
//! `model_returned`, `tool_called`, `tool_result` - carry that moment,
//! read from the clock the driver handed to [`Turn::begin`]; every other
//! line of the turn carries the turn's stamp. Order is `seq`'s business.
//!
//! `crates/runtime/spec/Turn.lean` proves what the boundaries must hold
//! (§8-3): a cancel before call k leaves exactly the first k calls
//! accounted, and a steer never ends a turn.

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

/// The typestate carrier. Phase data lives in `S` and is private to this
/// module: a phase literal cannot be forged, a phase cannot be skipped,
/// and no method returns an earlier phase. Everything the turn writes
/// goes through `journal`, which is the module's only ledger door.
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
    /// Opens a turn on the run's `lines`. `t` is the turn's stamp, which
    /// every line of this turn carries but the four it waited for; the
    /// executor advances it between turns (determinism rule 2). `now` is
    /// read for those four moments only, on this thread, and never by a
    /// tool. Lines an earlier turn held are still held, and go down at
    /// this turn's first barrier ahead of its own (runtime D36).
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

    /// Records how long this turn's model call and tool calls took, in
    /// microseconds read off `monotonic_us` (kernel D20). A turn opened
    /// without it records the moments and no durations.
    #[must_use]
    pub fn timed(mut self, monotonic_us: &'h mut dyn FnMut() -> u64) -> Turn<'h, Assembling> {
        self.journal.time_with(monotonic_us);
        self
    }

    /// Boundary 1 (before assembly). Builds the canonical request from
    /// the frozen prefix (system blocks), the window (messages) and the
    /// catalog's tool defs; appends `prompt_assembled` with the prefix's
    /// full source notes unless the run has already recorded that same
    /// payload.
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
        // The per-turn digest, recomputed from the bytes this request
        // will carry and checked against what the session froze. It
        // comes before `prompt_assembled` is written: a refusal must not
        // leave a line describing a prefix the city will not send.
        let segments = prompt.prefix.verified_segment_hashes()?;
        // One plan decides every breakpoint: the system blocks, the tail
        // message and the record all read it, so the record names only
        // breakpoints this request carries.
        let plan = crate::prefix::BreakpointPlan::for_conversation(conversation.messages());
        let payload = prompt.prefix.prompt_payload(&plan)?;
        if !prompt.recorded.holds(&payload) {
            self.journal
                .append_authored(Authored::PromptAssembled, payload.clone());
            prompt.recorded.remember(payload);
        }
        let chat = ChatRequest {
            model: shape.model.clone(),
            max_tokens: shape.max_tokens,
            system: prompt.prefix.system_blocks()?,
            messages: Cow::Borrowed(conversation.messages()),
            tools: Cow::Borrowed(tools),
            breakpoint: plan.message_breakpoint(),
            effort: shape.effort,
        };
        Ok(PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: Calling { segments, chat },
        }))
    }
}

impl<'h> Turn<'h, Calling<'_>> {
    /// Boundary 2 (before the provider call). Appends `model_called` and
    /// `model_returned`; a provider Err propagates after nothing but the
    /// boundary consumption touched the ledger. The reads `generating`
    /// starts early are not events: they ride to the wave, which uses
    /// each only for the settled call equal to the one started.
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
        generating: Generating<'_, 'sink>,
    ) -> Result<PhaseOutcome<Turn<'h, ToolWave>>, AxError> {
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
        // The door `generating` picks. Every door returns the same
        // `ModelReturn`, and the record below is written from that return
        // in each case - so what a page sees arriving and what the ledger
        // keeps cannot come from two different readings of one reply. A failure goes to the
        // recovery pipeline before it leaves this phase (`crates/runtime/spec/Turn/Recovery.lean`
        // §8-49), and every attempt - first or repaired - is recorded
        // before it is made.
        let mut call = recovery::ModelCall::open(&mut self.journal, ledger, model, &request);
        let mut repair = recovery::BlockingResend;
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
    /// Closes the turn. Recording is the accounting boundary: nothing
    /// extra is appended here, because every effect is already on the
    /// ledger. What the phase exists for is the fourth boundary — the
    /// last moment before the run acts on what this turn decided,
    /// including the work it handed down — and for the one compaction
    /// door a turn has. The wave is whole by now, so this is the one
    /// moment its snapshot may be replaced; a compactor meeting a
    /// half-landed wave would group what this boundary groups once, and
    /// `runtime::fork` rebuilding one turn at a time would answer
    /// different bytes for the same history.
    ///
    /// It pays no barrier: what the wave held waits in the run's lines
    /// for the next turn's model call, or for the run's freeze, and the
    /// report names `model_returned` by its [`Entry`] until then
    /// (runtime D36).
    ///
    /// # Errors
    /// Propagates the ledger's refusal to record the boundary event and
    /// a text the compaction cannot count.
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
        exchange.compact()?;
        Ok(PhaseOutcome::Advanced(TurnReport {
            redacted: self.journal.redacted(),
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
    /// Holds a line the run writes at `addr` between two phases, in its
    /// place among the turn's lines: it reaches the ledger at the next
    /// barrier with them, so the run's line costs no barrier of its own
    /// and the ledger's order is still the order of appending.
    pub(crate) fn hold_run_line(&mut self, line: RunLine, addr: Address, data: Payload) {
        self.journal.append_run_line(line, addr, data);
    }

    /// Ends the turn where it stands: `cancel_received`, one barrier that
    /// carries it with every line the run still held, and the refs of
    /// every line the run wrote so far.
    ///
    /// Reached from the boundary consumer below and from a wave that was
    /// halted between two calls, so both endings are one line written in
    /// one place.
    fn cancel_here(&mut self, ledger: &mut dyn Ledger) -> Result<TurnCancelled, AxError> {
        self.journal
            .append_authored(Authored::CancelReceived, Payload::of(&CancelReceived {})?);
        let lines = self.journal.lines();
        lines.barrier(ledger)?;
        Ok(TurnCancelled {
            refs: lines.refs().to_vec(),
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
            Interrupt::None | Interrupt::Policy { .. } => Ok(None),
            Interrupt::Cancel => Ok(Some(self.cancel_here(ledger)?)),
            Interrupt::Steer { source, text } => {
                let steer = SteerReceived { source, text };
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
