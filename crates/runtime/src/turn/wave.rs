// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Boundary 3: the tool wave, accounted in call order.
//!
//! The reads a turn started while the model was still generating
//! (`super::speculation`) are accounted here like every other call, each
//! taking its cached result; `crates/runtime/spec/Turn/Speculation.lean` is the
//! authority on why that leaves the lines, in the order and with the
//! payloads, that serial execution writes.
//!
//! Each call's two lines carry their own moments: the start is read once
//! the call is admitted and before its tool runs, the answer before the
//! tool face packages it, so a stamp the face renders is the answer's
//! moment (runtime D8). The turn reads both; the face holds none.
//!
//! A call that only reads runs without waiting for its `tool_called` to
//! be durable; a call with any other effect waits for its `tool_called`
//! and everything held before it to be durable before its tool runs, and
//! its `tool_result` rides the next barrier (runtime D24). Which
//! properties that order keeps on every crash point is
//! `crates/runtime/spec/Turn/Durability.lean`.

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{
    AxCode, AxError, ContentBlock, Effect, Ledger, ModelUsage, Payload, RenderIntent, TimeMs, Tool,
    ToolCall, ToolMeta, ToolOutcome,
};

use crate::bench::Ticket;

use crate::compaction::Exchange;

use super::ledger::Moment;
use super::{Carried, Interrupt, PhaseOutcome, Recording, ToolWave, Turn};

mod closure;
mod reorder;

use reorder::all_at_once;
pub(super) use reorder::lost_answer;

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
    /// The registration of the call's tool: its effect decides whether
    /// the call is read-only, and `tool_called` copies its effect and
    /// render (`crates/runtime/spec/Turn.lean` §8-51). `None` is a tool the bench does not
    /// know, which is not read-only.
    fn meta_of(&self, call: &ToolCall) -> Option<&ToolMeta>;
    /// The call behind a call through `call`, resolved before anything
    /// reads a registration (`crates/runtime/Spec.lean` §8-61).
    fn resolve_call(&self, call: ToolCall) -> ToolCall {
        call
    }
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

/// Whether the call's tool is registered as only reading.
pub(super) fn reads_only(tools: &dyn ConcurrentInvoke, call: &ToolCall) -> bool {
    matches!(tools.meta_of(call), Some(meta) if meta.effect == Effect::Read)
}

/// A call, the moment it started (after admission, before its tool
/// ran), and its tool's registration, which `tool_called` copies.
struct Intent<'c> {
    call: &'c ToolCall,
    started: Moment,
    effect: Option<Effect>,
    render: Option<RenderIntent>,
}

impl<'c> Intent<'c> {
    fn of(call: &'c ToolCall, started: Moment, tools: &dyn ConcurrentInvoke) -> Intent<'c> {
        let meta = tools.meta_of(call);
        Intent {
            call,
            started,
            effect: meta.map(|registered| registered.effect.clone()),
            render: meta.map(|registered| registered.render.clone()),
        }
    }
}

/// When one call started and when it answered: its `tool_result` is
/// stamped with the answer and carries the microseconds between the two.
#[derive(Clone, Copy)]
struct Timing {
    started: Moment,
    answered: Moment,
}

impl<'h> Turn<'h, ToolWave> {
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
    ) -> Result<PhaseOutcome<Turn<'h, Recording>>, AxError> {
        if let Some(cancelled) = self.consume_boundary(interrupt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        let calls: Vec<ToolCall> = std::mem::take(&mut self.state.calls)
            .into_iter()
            .map(|call| tools.resolve_call(call))
            .collect();
        let t = self.journal.stamp();
        let mut exchange = self.open_exchange();
        let reads = calls
            .iter()
            .take_while(|call| reads_only(tools, call))
            .count();
        let mut going = Vec::with_capacity(reads);
        let mut halt = Interrupt::None;
        for index in (0..=u32::MAX).take(reads) {
            match still_going(index) {
                Interrupt::Cancel => {
                    halt = Interrupt::Cancel;
                    break;
                }
                standing @ (Interrupt::None
                | Interrupt::Steer { .. }
                | Interrupt::Policy { .. }) => going.push(standing),
            }
        }
        let leading: Vec<&ToolCall> = calls.iter().take(going.len()).collect();
        let mut admitted = Vec::with_capacity(leading.len());
        let mut started = Vec::with_capacity(leading.len());
        for call in &leading {
            admitted.push(tools.admit(call, t));
            started.push(self.journal.read_moment()?);
        }
        let early = std::mem::take(&mut self.state.speculated).answers_for(&leading);
        let mut answers = all_at_once(&*tools, &leading, &admitted, &early).into_iter();
        for ((((call, at), standing), admission), cached) in leading
            .iter()
            .zip(started)
            .zip(going)
            .zip(admitted)
            .zip(early)
        {
            if let Some(cancelled) = self.consume_boundary(standing, ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            let answered_at = self.journal.read_moment()?;
            let answered = match admission {
                Admitted::Answered(answered) => answered,
                Admitted::Cleared(ticket) => {
                    let ran = cached
                        .unwrap_or_else(|| answers.next().unwrap_or_else(|| Err(lost_answer())));
                    tools.account(call, ticket, ran)
                }
            };
            self.intend(Intent::of(call, at, &*tools))?;
            let timing = Timing {
                started: at,
                answered: answered_at,
            };
            self.settle(&mut exchange, call, timing, answered)?;
        }
        if let Some(cancelled) = self.consume_boundary(halt, ledger)? {
            return Ok(PhaseOutcome::Cancelled(cancelled));
        }
        for (index, call) in (0..=u32::MAX).zip(&calls).skip(leading.len()) {
            if let Some(cancelled) = self.consume_boundary(still_going(index), ledger)? {
                return Ok(PhaseOutcome::Cancelled(cancelled));
            }
            self.alone(ledger, &mut exchange, tools, call)?;
        }
        Ok(self.recorded(calls, exchange))
    }

    fn open_exchange(&mut self) -> Exchange {
        let mut exchange = Exchange::new();
        exchange.push_assistant(std::mem::take(&mut self.state.assistant));
        exchange
    }

    /// One call through all three stages on this thread: how every call
    /// after the leading reads runs. The start is read once the call is
    /// admitted; the answer before the face's `account` packages it. A
    /// cleared call that does not only read is an outside effect, so its
    /// `tool_called` and everything held before it are made durable before
    /// its tool runs: the intent is on the ledger first.
    fn alone(
        &mut self,
        ledger: &mut dyn Ledger,
        exchange: &mut Exchange,
        tools: &mut dyn ConcurrentInvoke,
        call: &ToolCall,
    ) -> Result<(), AxError> {
        let admission = tools.admit(call, self.journal.stamp());
        let started = self.journal.read_moment()?;
        self.intend(Intent::of(call, started, &*tools))?;
        let (answered_at, answered) = match admission {
            Admitted::Answered(answered) => (self.journal.read_moment()?, answered),
            Admitted::Cleared(ticket) => {
                if !reads_only(&*tools, call) {
                    self.journal.barrier(ledger)?;
                }
                let ran = tools.tool(&ticket).and_then(|tool| tool.invoke(call));
                let answered_at = self.journal.read_moment()?;
                (answered_at, tools.account(call, ticket, ran))
            }
        };
        let timing = Timing {
            started,
            answered: answered_at,
        };
        self.settle(exchange, call, timing, answered)
    }

    /// Appends one call's `tool_called` line.
    fn intend(&mut self, intent: Intent<'_>) -> Result<(), AxError> {
        let Intent {
            call,
            started,
            effect,
            render,
        } = intent;
        let called = ToolCalled {
            id: call.id.clone(),
            name: call.name.clone(),
            subject: ToolCalled::subject_of(&call.args),
            args: call.args.clone(),
            effect,
            render,
        };
        self.journal.append_redacted(
            Carried::ToolCalled { at: started.at },
            Payload::of(&called)?,
        )?;
        Ok(())
    }

    /// Appends one call's `tool_result` line and pushes its result block:
    /// the one place a wave's answers are accounted, whichever way the
    /// call was run.
    fn settle(
        &mut self,
        exchange: &mut Exchange,
        call: &ToolCall,
        timing: Timing,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<(), AxError> {
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
            took_us: timing.answered.since(timing.started.us),
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
        self.journal.append_redacted(
            Carried::ToolResult {
                at: timing.answered.at,
            },
            Payload::of(&result)?,
        )?;
        Ok(())
    }

    fn recorded(
        self,
        calls: Vec<ToolCall>,
        exchange: Exchange,
    ) -> PhaseOutcome<Turn<'h, Recording>> {
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
