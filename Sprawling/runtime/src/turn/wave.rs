// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub trait ConcurrentInvoke {
    fn meta_of(&self, call: &ToolCall) -> Option<&ToolMeta>;

    fn resolve_call(&self, call: ToolCall) -> ToolCall {
        call
    }

    fn ahead(&self, _call: &ToolCall) -> Option<&dyn Tool> {
        None
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted;

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError>;

    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError>;

    fn complete_wave(&mut self, _shortened: &std::collections::BTreeSet<String>) {}
}

#[derive(Debug)]
pub enum Admitted {
    Answered(Result<ToolOutcome, AxError>),

    Cleared(Ticket),
}

pub(super) fn reads_only(tools: &dyn ConcurrentInvoke, call: &ToolCall) -> bool {
    matches!(tools.meta_of(call), Some(meta) if meta.effect == Effect::Read)
}

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

#[derive(Clone, Copy)]
struct Timing {
    started: Moment,
    answered: Moment,
}

impl<'h> Turn<'h, ToolWave> {
    pub(crate) fn calls(&self) -> &[ToolCall] {
        &self.state.calls
    }

    pub(crate) fn usage(&self) -> Option<&ModelUsage> {
        self.state.usage.as_ref()
    }

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

    fn settle(
        &mut self,
        exchange: &mut Exchange,
        call: &ToolCall,
        timing: Timing,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<(), AxError> {
        let mut pictures = Vec::new();
        let (answer, is_error) = match answered {
            Ok(ToolOutcome {
                result,
                attachments,
            }) => {
                pictures = attachments;
                (ToolAnswer::Answered { result }, false)
            }
            Err(tool_err) => (
                ToolAnswer::Failed {
                    error: Payload::of(&tool_err)?,
                },
                true,
            ),
        };

        let content = crate::rendering::shown(crate::rendering::CURRENT, &call.name, &answer)?;
        let result = ToolResult {
            tool_use_id: call.id.clone(),
            name: call.name.clone(),
            answer,
            took_us: timing.answered.since(timing.started.us),
            rendered: Some(crate::rendering::CURRENT),
            shown_bytes: Some(u64::try_from(content.len()).map_err(|_| {
                AxError::failure(AxCode::InvalidArgs, "count a shown answer", call.id.clone())
                    .with_recovery("this machine cannot count the answer's bytes in a u64")
            })?),

            attachments: pictures.clone(),
        };
        exchange.push_result(ContentBlock::ToolResult {
            tool_use_id: call.id.clone(),
            content,
            is_error,

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
