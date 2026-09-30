// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, accounted in call order.
//!
//! The reads a turn started while the model was still generating
//! (`super::speculation`) are accounted here like every other call, each
//! taking its cached result; `tools/adversary/design/Speculating.lean` is the
//! authority on why that leaves the Ledger serial execution writes.

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{
    AxCode, AxError, ContentBlock, Effect, Ledger, ModelUsage, Payload, TimeMs, Tool, ToolCall,
    ToolOutcome,
};

use crate::bench::Ticket;

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

/// A wave's tools in three stages, so the calls that only read can run
/// at once while everything they decide stays in call order.
///
/// The leading run of calls whose tool declares `Effect::Read` is
/// admitted one call at a time, in call order; the tools of the calls it
/// clears then run at once, each on its own scoped thread; and every
/// answer is accounted one call at a time, in call order. The first
/// call with any other effect waits until every one of them has
/// answered, and from there the wave is serial. The Ledger sees the same
/// lines in the same order either way: results wait in a reorder buffer
/// and are written in call order.
///
/// A trait rather than three closures: admitting and accounting write
/// the same dedup table and taint that `tool` lends out, and three
/// closures cannot each borrow one bench.
pub trait ConcurrentInvoke {
    /// The effect the call's tool declares. `None` is a tool the
    /// catalog does not know, which is not read-only.
    fn effect_of(&self, call: &ToolCall) -> Option<Effect>;
    /// The tool a read-only call would run on, lent out before the call
    /// is admitted so it can start while the model is still generating.
    /// What it returns reaches the model only if `admit` later clears the
    /// same call. `None`, the default, starts nothing early.
    fn ahead(&self, _call: &ToolCall) -> Option<&dyn Tool> {
        None
    }
    /// Stage one, serial in call order: whether the call may run, at the
    /// turn's stamp. An answer that needs no tool (a replay, a door's
    /// refusal) comes back here.
    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted;
    /// Stage two: the tool a cleared call runs on. It takes `&self`, so
    /// every cleared call's tool is lent out at once and invoked from
    /// several threads; a tool is `Sync` where the implementor need not be.
    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError>;
    /// Stage three, serial in call order: records what the tool answered
    /// and returns what the model reads back.
    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError>;
}

/// What admitting one call decided.
#[derive(Debug)]
pub enum Admitted {
    /// Answered without running a tool; the answer goes to the model.
    Answered(Result<ToolOutcome, AxError>),
    /// The call may run: `tool` and then `account` take the ticket.
    Cleared(Ticket),
}

/// A closure is a tool face that answers every call as it admits it:
/// it declares no effect, so its waves are serial. citysim and the tests
/// that script a tool's answer drive a run through this one.
impl<F> ConcurrentInvoke for F
where
    F: FnMut(&ToolCall, TimeMs) -> Result<ToolOutcome, AxError>,
{
    fn effect_of(&self, _call: &ToolCall) -> Option<Effect> {
        None
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        Admitted::Answered(self(call, t))
    }

    fn tool(&self, _ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        Err(unheld_ticket())
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        _ticket: Ticket,
        _answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        Err(unheld_ticket())
    }
}

fn unheld_ticket() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "run a cleared tool call",
        "a tool face that answers every call as it admits it was handed a ticket",
    )
    .with_recovery("report this against runtime::turn::wave: only a bench issues tickets")
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

    /// Boundary 3 (before tool execution), accounted in call order, with
    /// the leading read-only calls run at once (see [`ConcurrentInvoke`]).
    /// A tool Err is not a turn Err — it lands in `tool_result` and goes
    /// back to the model (the model is the recovery subject).
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
    /// hands it to the model. For the leading reads it is asked for every
    /// call before any of them starts, in call order, and a cancel starts
    /// only the calls before it: the calls a serial wave would have made
    /// before the same cancel. Each answer is consumed just before its
    /// call is accounted, so a steer lands on the ledger where a serial
    /// wave writes it.
    pub fn execute_concurrent(
        mut self,
        interrupt: Interrupt,
        ledger: &mut dyn Ledger,
        tools: &mut dyn ConcurrentInvoke,
        still_going: &mut dyn FnMut(u32) -> Interrupt,
    ) -> Result<PhaseOutcome<Turn<Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls = std::mem::take(&mut self.state.calls);
        let t = self.journal.stamp();
        let mut exchange = self.open_exchange();
        let reads = calls
            .iter()
            .take_while(|call| tools.effect_of(call) == Some(Effect::Read))
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
        let admitted: Vec<Admitted> = leading.iter().map(|call| tools.admit(call, t)).collect();
        let early = std::mem::take(&mut self.state.speculated).answers_for(&leading);
        let mut answers = all_at_once(&*tools, &leading, &admitted, &early).into_iter();
        for (((call, standing), admission), cached) in
            leading.iter().zip(going).zip(admitted).zip(early)
        {
            if let Some(cancelled) = self.consume_boundary(standing, ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            let answered = match admission {
                Admitted::Answered(answered) => answered,
                Admitted::Cleared(ticket) => {
                    let ran = cached
                        .unwrap_or_else(|| answers.next().unwrap_or_else(|| Err(lost_answer())));
                    tools.account(call, ticket, ran)
                }
            };
            self.account(ledger, &mut exchange, call, answered)?;
        }
        if let Some(cancelled) = self.consume_boundary(halt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        for (index, call) in (0..=u32::MAX).zip(&calls).skip(leading.len()) {
            if let Some(cancelled) = self.consume_boundary(still_going(index), ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            let answered = alone(tools, call, t);
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

/// One call through all three stages on this thread: how every call
/// after the leading reads runs.
fn alone(
    tools: &mut dyn ConcurrentInvoke,
    call: &ToolCall,
    t: TimeMs,
) -> Result<ToolOutcome, AxError> {
    match tools.admit(call, t) {
        Admitted::Answered(answered) => answered,
        Admitted::Cleared(ticket) => {
            let ran = tools.tool(&ticket).and_then(|tool| tool.invoke(call));
            tools.account(call, ticket, ran)
        }
    }
}

/// Runs the tool of every cleared call that holds no answer from while
/// the model was generating, all at once, and answers in call order, one
/// answer per such call: the reorder buffer. The first runs on this
/// thread, so a wave of one read starts no thread at all. The scope is
/// this module's exception to the one spawn point (ARCHITECTURE §10
/// rule 3): every thread it starts is joined before it returns.
fn all_at_once(
    tools: &dyn ConcurrentInvoke,
    calls: &[&ToolCall],
    admitted: &[Admitted],
    early: &[Option<Result<ToolOutcome, AxError>>],
) -> Vec<Result<ToolOutcome, AxError>> {
    let running: Vec<(&ToolCall, Result<&dyn Tool, AxError>)> = calls
        .iter()
        .zip(admitted)
        .zip(early)
        .filter_map(|((call, admission), cached)| match (admission, cached) {
            (Admitted::Cleared(ticket), None) => Some((*call, tools.tool(ticket))),
            (Admitted::Cleared(_), Some(_)) | (Admitted::Answered(_), _) => None,
        })
        .collect();
    let run = |(call, tool): &(&ToolCall, Result<&dyn Tool, AxError>)| match tool {
        Ok(tool) => tool.invoke(call),
        Err(unheld) => Err(unheld.clone()),
    };
    let Some((first, rest)) = running.split_first() else {
        return Vec::new();
    };
    std::thread::scope(|scope| {
        let others: Vec<_> = rest
            .iter()
            .map(|job| scope.spawn(move || run(job)))
            .collect();
        std::iter::once(run(first))
            .chain(
                others
                    .into_iter()
                    .map(|worker| worker.join().unwrap_or_else(|_| Err(lost_answer()))),
            )
            .collect()
    })
}

pub(super) fn lost_answer() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "run a read-only tool call",
        "the tool stopped its thread without an answer",
    )
    .with_recovery("call the tool again; report it when it stops a second time")
}
