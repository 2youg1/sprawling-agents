// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, accounted in call order.

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{
    AxCode, AxError, ContentBlock, Effect, Ledger, ModelUsage, Payload, ToolCall, ToolOutcome,
};

use crate::compaction::Exchange;

use super::{Carried, Interrupt, PhaseOutcome, Recording, ToolWave, Turn};

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

/// A wave's invoker that may be called from several threads at once,
/// with the lookup that says which calls only read.
///
/// The leading run of calls whose tool declares `Effect::Read` runs at
/// once; the first call with any other effect waits until every one of
/// them has answered, and from there the wave is serial. The Ledger sees
/// the same lines in the same order either way: results wait in a
/// reorder buffer and are written in call order.
pub struct ConcurrentInvoke<'a> {
    pub invoke: &'a (dyn Fn(&ToolCall) -> Result<ToolOutcome, AxError> + Sync),
    /// `None` is a tool the catalog does not know, which is not read-only.
    pub effect_of: &'a dyn Fn(&ToolCall) -> Option<Effect>,
}

impl Turn<ToolWave> {
    /// The calls this wave is about to make, in call order.
    pub(crate) fn calls(&self) -> &[ToolCall] {
        &self.state.calls
    }

    /// What the provider counted for the call that asked for this wave.
    pub(crate) fn usage(&self) -> Option<&ModelUsage> {
        self.state.usage.as_ref()
    }

    /// Boundary 3 (before tool execution), serial: accounting order is
    /// the call order. A tool Err is not a turn Err — it lands in
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
    /// the person who stopped the city waited for all of them. What it
    /// answers is consumed through the one boundary consumer, so a steer
    /// between two calls is recorded here, before the next assembly
    /// hands it to the model.
    pub fn execute(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        invoke: &mut dyn FnMut(&ToolCall) -> Result<ToolOutcome, AxError>,
        still_going: &mut dyn FnMut(u32) -> Interrupt,
    ) -> Result<PhaseOutcome<Turn<Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls = std::mem::take(&mut self.state.calls);
        let mut exchange = self.open_exchange();
        for (index, call) in (0..=u32::MAX).zip(&calls) {
            // Asked before the call is written down, so a wave that was
            // stopped leaves no `tool_called` line for work nothing ever
            // did. Through the same door a phase boundary takes, so a
            // wave that stops leaves the one `cancel_received` line every
            // other ending leaves, and a steer its `steer_received`.
            if let Some(cancelled) = self.consume_boundary(still_going(index), ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            let answered = invoke(call);
            self.account(ledger, &mut exchange, call, answered)?;
        }
        Ok(self.recorded(calls, exchange))
    }

    /// Boundary 3 with the leading read-only calls run at once (see
    /// [`ConcurrentInvoke`]). `still_going` is asked for every call of
    /// that leading run before any of them starts, in call order, and a
    /// cancel starts only the calls before it: the calls a serial wave
    /// would have made before the same cancel. Each answer is consumed
    /// just before its call is accounted, so a steer lands on the ledger
    /// where the serial wave writes it.
    pub fn execute_concurrent(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        tools: &ConcurrentInvoke<'_>,
        still_going: &mut dyn FnMut(u32) -> Interrupt,
    ) -> Result<PhaseOutcome<Turn<Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls = std::mem::take(&mut self.state.calls);
        let mut exchange = self.open_exchange();
        let reads = calls
            .iter()
            .take_while(|call| (tools.effect_of)(call) == Some(Effect::Read))
            .count();
        let mut going = Vec::with_capacity(reads);
        let mut halt = Interrupt::None;
        for index in (0..=u32::MAX).take(reads) {
            match still_going(index) {
                Interrupt::Cancel => {
                    halt = Interrupt::Cancel;
                    break;
                }
                standing @ (Interrupt::None | Interrupt::Steer { .. }) => going.push(standing),
            }
        }
        let leading: Vec<&ToolCall> = calls.iter().take(going.len()).collect();
        let answers = all_at_once(&leading, tools.invoke);
        for ((call, standing), answered) in leading.iter().zip(going).zip(answers) {
            if let Some(cancelled) = self.consume_boundary(standing, ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            self.account(ledger, &mut exchange, call, answered)?;
        }
        if let Some(cancelled) = self.consume_boundary(halt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        for (index, call) in (0..=u32::MAX).zip(&calls).skip(leading.len()) {
            if let Some(cancelled) = self.consume_boundary(still_going(index), ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            let answered = (tools.invoke)(call);
            self.account(ledger, &mut exchange, call, answered)?;
        }
        Ok(self.recorded(calls, exchange))
    }

    fn open_exchange(&mut self) -> Exchange {
        let mut exchange = Exchange::new();
        exchange.push_assistant(std::mem::take(&mut self.state.assistant));
        exchange
    }

    /// Writes one call's `tool_called` and `tool_result` lines and its
    /// result block: the one place a wave's accounting happens, whichever
    /// way the call was run.
    fn account(
        &mut self,
        ledger: &mut dyn Ledger,
        exchange: &mut Exchange,
        call: &ToolCall,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<(), AxError> {
        let called = ToolCalled {
            id: call.id.clone(),
            name: call.name.clone(),
            subject: ToolCalled::subject_of(&call.args),
            args: call.args.clone(),
        };
        self.journal
            .append_redacted(ledger, Carried::ToolCalled, Payload::of(&called)?)?;
        let mut pictures = Vec::new();
        let (answer, content, is_error) = match answered {
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
        exchange.push_result(ContentBlock::ToolResult {
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
        Ok(())
    }

    fn recorded(self, calls: Vec<ToolCall>, exchange: Exchange) -> PhaseOutcome<Turn<Recording>> {
        PhaseOutcome::Advanced(Turn {
            journal: self.journal,
            state: Recording {
                model_returned: self.state.model_returned,
                calls_made: calls.len(),
                exchange,
                usage: self.state.usage,
                stop: self.state.stop,
            },
        })
    }
}

/// Runs every call at once and answers in call order: the reorder
/// buffer. The first call runs on this thread, so a wave of one read
/// starts no thread at all. The scope is this module's exception to the
/// one spawn point (ARCHITECTURE §10 rule 3): every thread it starts is
/// joined before it returns.
fn all_at_once(
    calls: &[&ToolCall],
    invoke: &(dyn Fn(&ToolCall) -> Result<ToolOutcome, AxError> + Sync),
) -> Vec<Result<ToolOutcome, AxError>> {
    let Some((first, rest)) = calls.split_first() else {
        return Vec::new();
    };
    std::thread::scope(|scope| {
        let others: Vec<_> = rest
            .iter()
            .map(|call| scope.spawn(move || invoke(call)))
            .collect();
        std::iter::once(invoke(first))
            .chain(others.into_iter().map(|worker| {
                worker.join().unwrap_or_else(|_| {
                    Err(AxError::failure(
                        AxCode::ToolUnavailable,
                        "run a read-only tool call",
                        "the tool stopped its thread without an answer",
                    )
                    .with_recovery("call the tool again; report it when it stops a second time"))
                })
            }))
            .collect()
    })
}
