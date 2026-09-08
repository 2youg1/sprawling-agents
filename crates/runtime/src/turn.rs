// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The turn typestate: Assembling -> Calling ->
//! ToolWave -> Recording, phase changes carrying `&mut dyn Ledger`.
//! Interrupts are consumed *only* at phase boundaries — the boundary
//! snapshot is a parameter of every transition, and no method exists that
//! could observe one mid-phase. That absence is A9's structural half.
//! Steer consumes at a boundary too, but advances: it is an addition to
//! the window, not an ending.
//!
//! All events of one turn share the timestamp given to [`Turn::begin`]:
//! order is `seq`'s business, time is a parameter, never sampled.

use kernel::{
    AxCode, AxError, B3Hash, BuildingPolicy, ChatRequest, ContentBlock, EventDraft, EventKind,
    EventRef, Ledger, Model, ModelRequest, ModelReturn, ModelUsage, Payload, RunId, TimeMs,
    ToolCall, ToolDef, content_from_message,
};
use serde_json::{Map, Value};

use crate::prefix::FrozenPrefix;
use crate::window::Window;

mod boundary;
mod report;
mod wave;

pub use boundary::{Interrupt, PhaseOutcome, TurnCancelled};
pub use report::{CallShape, TurnReport};

/// The typestate carrier. Phase data lives in `S` and is private to this
/// module: a phase literal cannot be forged, a phase cannot be skipped,
/// and no method returns an earlier phase.
#[derive(Debug)]
pub struct Turn<S> {
    run: RunId,
    who: String,
    t: TimeMs,
    refs: Vec<EventRef>,
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
}

#[derive(Debug)]
pub struct Recording {
    model_returned: EventRef,
    calls_made: usize,
    assistant: Vec<ContentBlock>,
    wave_results: Vec<ContentBlock>,
    usage: Option<ModelUsage>,
}

fn payload(map: Map<String, Value>) -> Result<Payload, AxError> {
    Payload::new(map)
}

impl Turn<Assembling> {
    /// Opens a turn. `t` stamps every event of this turn; the executor
    /// advances it between turns (determinism rule 2).
    pub fn begin(run: RunId, who: String, t: TimeMs) -> Turn<Assembling> {
        Turn {
            run,
            who,
            t,
            refs: Vec::new(),
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
        let prompt = prefix.prompt_payload()?;
        let echo = ledger.append(self.draft(EventKind::PromptAssembled, prompt))?;
        self.refs.push(echo);
        let chat = ChatRequest {
            model: shape.model.clone(),
            max_tokens: shape.max_tokens,
            system: prefix.system_blocks()?,
            messages: window.messages().to_vec(),
            tools: tools.to_vec(),
            effort: shape.effort,
        };
        Ok(PhaseOutcome::Advanced(Turn {
            run: self.run,
            who: self.who,
            t: self.t,
            refs: self.refs,
            state: Calling {
                segments: prefix.segment_hashes(),
                chat,
            },
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
        deltas: Option<&mut (dyn FnMut(&str) + 'sink)>,
    ) -> Result<PhaseOutcome<Turn<ToolWave>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let Calling { segments, chat } = self.state;
        let request = ModelRequest {
            policy: policy.clone(),
            segments,
            chat,
        };
        let mut called = Map::new();
        called.insert(
            "segments".to_owned(),
            Value::Array(
                request
                    .segments
                    .iter()
                    .map(|hash| Value::String(hash.to_string()))
                    .collect(),
            ),
        );
        called.insert(
            "model".to_owned(),
            Value::String(request.chat.model.clone()),
        );
        let echo = ledger.append(EventDraft {
            run: self.run,
            t: self.t,
            who: self.who.clone(),
            addr: None,
            kind: EventKind::ModelCalled,
            data: payload(called)?,
            ig: false,
        })?;
        self.refs.push(echo);
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
        let mut returned = Map::new();
        returned.insert(
            "message".to_owned(),
            serde_json::to_value(&message).map_err(|err| {
                AxError::failure(AxCode::InvalidArgs, "encode model message", err.to_string())
            })?,
        );
        let calls_len = u64::try_from(calls.len()).map_err(|_| {
            AxError::failure(AxCode::InvalidArgs, "encode model return", "wave too large")
        })?;
        returned.insert("calls".to_owned(), Value::Number(calls_len.into()));
        if let Some(usage) = &usage {
            returned.insert(
                "usage".to_owned(),
                serde_json::to_value(usage).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode usage", err.to_string())
                })?,
            );
        }
        if let Some(stop) = &stop {
            returned.insert(
                "stop".to_owned(),
                serde_json::to_value(stop).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode stop reason", err.to_string())
                })?,
            );
        }
        if let Some(billed) = billed_usd_micros {
            returned.insert(
                "billed_usd_micros".to_owned(),
                Value::Number(billed.get().into()),
            );
        }
        // The window already holds the blocks this turn will send back,
        // so redacting here cannot break a thinking block's signature.
        // What it does stop is a key the model repeated from becoming a
        // permanent, exportable line of history.
        let (returned, _redacted) = crate::redact::redact(&returned);
        let model_returned = ledger.append(EventDraft {
            run: self.run,
            t: self.t,
            who: self.who.clone(),
            addr: None,
            kind: EventKind::ModelReturned,
            data: payload(returned)?,
            ig: false,
        })?;
        self.refs.push(model_returned);
        Ok(PhaseOutcome::Advanced(Turn {
            run: self.run,
            who: self.who,
            t: self.t,
            refs: self.refs,
            state: ToolWave {
                calls,
                model_returned,
                assistant,
                usage,
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
            refs: self.refs,
            model_returned: self.state.model_returned,
            calls_made: self.state.calls_made,
            assistant: self.state.assistant,
            wave_results: self.state.wave_results,
            usage: self.state.usage,
        }))
    }
}

impl<S> Turn<S> {
    fn draft(&self, kind: EventKind, data: Payload) -> EventDraft {
        EventDraft {
            run: self.run,
            t: self.t,
            who: self.who.clone(),
            addr: None,
            kind,
            data,
            ig: false,
        }
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
            Interrupt::Cancel => {
                let echo =
                    ledger.append(self.draft(EventKind::CancelReceived, Payload::empty()))?;
                self.refs.push(echo);
                let mut refs = std::mem::take(&mut self.refs);
                refs.shrink_to_fit();
                Ok(Some(TurnCancelled { refs }))
            }
            Interrupt::Steer { source, text } => {
                let mut map = Map::new();
                map.insert("source".to_owned(), Value::String(source));
                map.insert("text".to_owned(), Value::String(text));
                let echo = ledger.append(self.draft(EventKind::SteerReceived, payload(map)?))?;
                self.refs.push(echo);
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
    reason = "test code"
)]
mod tests;
