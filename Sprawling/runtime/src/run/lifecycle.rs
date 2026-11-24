// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    AxError, Carrier, Completion, EventDraft, Evidence, Ledger, Model, Payload, StopReason, TimeMs,
};

use crate::conversation::Conversation;
use crate::handoff::Handoff;
use crate::mode::{PolicyCell, policy_note};
use crate::prefix::shape::PromptShape;
use crate::reminder::ContextGauge;
use crate::turn::{
    Generating, HeldLines, Interrupt, PhaseOutcome, RunLine, RunPrompt, Turn, TurnReport,
};

use super::charter::Verdict;
use super::checkpoint::{CheckpointPolicy, Wave, WaveCheckpoint};
use super::{Active, Advance, Frozen, Run, RunHooks, RunPlan, SafePoint};

fn fold_steer(conversation: &mut Conversation, interrupt: &Interrupt) {
    if let Interrupt::Steer {
        speaker,
        text,
        pictures,
        ..
    } = interrupt
    {
        conversation.push_steer(speaker, text, pictures);
    }
}

fn hold_policy(cell: &mut PolicyCell, interrupt: &Interrupt) {
    if let Interrupt::Policy { policy } = interrupt {
        cell.arrive(*policy);
    }
}

fn fold_arrival(conversation: &mut Conversation, policy: &mut PolicyCell, interrupt: &Interrupt) {
    fold_steer(conversation, interrupt);
    hold_policy(policy, interrupt);
}

fn concluded(report: &TurnReport, lines: &HeldLines) -> Result<Completion, AxError> {
    if report.assistant().is_empty() || report.stop() == Some(StopReason::MaxTokens) {
        return Ok(Completion::Limit);
    }
    Ok(Completion::Done(Evidence::new(vec![
        lines.durable(report.model_returned())?,
    ])?))
}

impl Run<Active> {
    pub(super) fn line(&self, t: TimeMs, kind: kernel::EventKind, data: Payload) -> EventDraft {
        EventDraft {
            run: self.plan.run,
            t,
            who: self.plan.who.clone(),
            addr: Some(self.plan.addr.clone()),
            kind,
            data,
            ig: false,
        }
    }

    pub(super) fn record_fired(
        &self,
        ledger: &mut dyn Ledger,
        t: TimeMs,
        disposal: &crate::Disposal,
    ) -> Result<(), AxError> {
        let data = self.state.watchdog.fired_payload(disposal)?;
        ledger
            .append(self.line(t, kernel::EventKind::WatchdogFired, data))
            .map(|_| ())
    }

    pub(super) fn record_refusal(
        &self,
        ledger: &mut dyn Ledger,
        t: TimeMs,
        refusal: &AxError,
    ) -> Result<(), AxError> {
        let Carrier::Event(kind) = refusal.code().carrier() else {
            return Err(refusal.clone());
        };
        ledger
            .append(self.line(t, kind, Payload::of(refusal)?))
            .map(|_| ())
    }

    pub fn dispatch(
        plan: RunPlan,
        ledger: &mut dyn Ledger,
        hooks: &mut RunHooks<'_>,
    ) -> Result<Run<Active>, AxError> {
        plan.charter().open(ledger, &mut *hooks.now)?;
        let lines = HeldLines::open(plan.run, plan.who.clone());

        let mut conversation = Conversation::new();

        conversation.push_inherited(&plan.inherited);
        conversation.push_task_lines(
            (&plan.task, &plan.goal),
            plan.opening,
            &plan
                .dispatched_by
                .handed_down_by(plan.predecessor, plan.parent),
            &plan.pictures,
        );
        let model_window = kernel::Tokens::new(plan.shape.context_tokens);
        let (window, locked) = match plan.context_lock {
            Some(lock) => (lock.effective(model_window), true),
            None => (model_window, false),
        };
        let gauge = ContextGauge::locked(window, plan.second_threshold, locked);
        let watchdog = crate::Watchdog::new(plan.retries, plan.run);
        Ok(Run {
            plan,
            state: Active {
                conversation,
                turns: 0,
                last_turn_t: None,
                failure: None,
                gauge,
                prior_shape: None,
                prompt: crate::turn::PromptRecord::default(),
                checkpoint: CheckpointPolicy::opening(),
                lines,
                watchdog,
            },
        })
    }

    pub fn advance(
        &mut self,
        ledger: &mut dyn Ledger,
        model: &mut dyn Model,
        hooks: &mut RunHooks<'_>,
    ) -> Result<Advance, AxError> {
        let index = self.state.turns;
        let t = (hooks.now)()?;
        self.state.last_turn_t = Some(t);

        let turn =
            Turn::begin(&mut self.state.lines, t, &mut *hooks.now).timed(&mut *hooks.monotonic_us);
        let opening = (hooks.interrupt)(SafePoint::BeforeAssemble { turn: index });
        fold_arrival(
            &mut self.state.conversation,
            &mut self.plan.run_policy,
            &opening,
        );
        let mut turn = match turn.assemble(
            opening,
            ledger,
            RunPrompt::new(&self.plan.prefix, &mut self.state.prompt),
            &self.state.conversation,
            &self.plan.tools,
            &self.plan.shape,
        )? {
            PhaseOutcome::Advanced(next) => next,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };

        let shape = PromptShape::of(
            &self.plan.prefix,
            &self.plan.tools,
            self.state.conversation.messages(),
        )?;
        let changed = shape.attribute(self.state.prior_shape.as_ref());
        turn.hold_run_line(
            RunLine::PromptShapeCompared,
            self.plan.addr.clone(),
            Payload::of(&shape.recorded(changed)?)?,
        );
        self.state.prior_shape = Some(shape);
        let calling = (hooks.interrupt)(SafePoint::BeforeCall { turn: index });
        let called = turn.under(&mut self.state.watchdog).call(
            calling.clone(),
            ledger,
            model,
            &self.plan.policy,
            Generating::Speculating {
                deltas: hooks.deltas.as_deref_mut(),
                tools: &*hooks.invoke,
            },
        );

        self.state.conversation.mark_sent();
        fold_arrival(
            &mut self.state.conversation,
            &mut self.plan.run_policy,
            &calling,
        );
        let mut turn = match called? {
            PhaseOutcome::Advanced(next) => next,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };

        if let Some(usage) = turn.usage() {
            self.plan.context.record(usage.input_tokens);
        }

        let touches = Wave::of(turn.calls(), hooks.writes);
        let decided = self.state.checkpoint.for_wave(touches);
        if let (WaveCheckpoint::Stage, Some(checkpoint)) = (decided, hooks.checkpoint.as_mut()) {
            let committed = match checkpoint(t) {
                Ok(committed) => committed,
                Err(failure) => {
                    if let Some(refusal) = failure.refusal {
                        turn.hold_run_line(
                            RunLine::SecretEgressBlocked,
                            self.plan.addr.clone(),
                            Payload::of(&refusal)?,
                        );
                    }
                    return Err(failure.cause);
                }
            };
            turn.hold_run_line(
                RunLine::CheckpointCommitted,
                self.plan.addr.clone(),
                committed,
            );
        }

        self.state.checkpoint.record_wave(decided, touches);

        let wave = (hooks.interrupt)(SafePoint::BeforeWave { turn: index });
        fold_arrival(
            &mut self.state.conversation,
            &mut self.plan.run_policy,
            &wave,
        );

        let taken = self.plan.run_policy.take_at_wave();

        let asking = &mut hooks.interrupt;
        let state = &mut self.state.conversation;
        let cell = &mut self.plan.run_policy;
        let mut still_going = |call: u32| {
            let arrived = asking(SafePoint::BeforeToolCall { turn: index, call });
            fold_arrival(state, cell, &arrived);
            arrived
        };
        let turn =
            match turn.execute_concurrent(wave, ledger, &mut *hooks.invoke, &mut still_going)? {
                PhaseOutcome::Advanced(next) => next,
                PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
            };

        let settling = (hooks.interrupt)(SafePoint::BeforeSpawn { turn: index });
        fold_arrival(
            &mut self.state.conversation,
            &mut self.plan.run_policy,
            &settling,
        );
        let report = match turn.record(settling, ledger)? {
            PhaseOutcome::Advanced(report) => report,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };

        hooks.invoke.complete_wave(report.shortened());
        self.state.turns = self.state.turns.saturating_add(1);
        self.state
            .conversation
            .push_assistant(report.assistant().to_vec());
        self.state
            .conversation
            .push_tool_results(report.wave_results().to_vec());

        if let Some(policy) = taken {
            self.state.conversation.push_steer(
                &crate::conversation::Speaker::City,
                &policy_note(&policy),
                &[],
            );
        }

        if let Some(reminder) = report
            .usage()
            .and_then(|usage| self.state.gauge.observe(usage.input_tokens))
        {
            self.state.conversation.push_reminder(&reminder);

            if reminder.asks_succession()
                && report.calls_made() > 0
                && let Ok(mut desk) = self.plan.succession.lock()
            {
                desk.ask(crate::Succession {
                    reason: "context lock reached".to_owned(),
                });
            }
        }
        if report.calls_made() == 0 {
            self.state.lines.barrier(ledger)?;
            return Ok(Advance::Concluded(concluded(&report, &self.state.lines)?));
        }
        Ok(Advance::Turned)
    }

    pub fn freeze(
        self,
        ledger: &mut dyn Ledger,
        handoff: &Handoff,
        completion: Completion,
        hooks: &mut RunHooks<'_>,
    ) -> Result<Run<Frozen>, AxError> {
        let mut lines = self.state.lines;
        lines.barrier(ledger)?;
        let t = match (&completion, self.state.last_turn_t) {
            (Completion::Cancelled, Some(turn_t)) => turn_t,
            _ => (hooks.now)()?,
        };
        self.plan.charter().close(
            ledger,
            handoff,
            Verdict {
                completion: &completion,
                failure: self.state.failure.as_ref(),
            },
            t,
        )?;
        let turns = self.state.turns;
        Ok(Run {
            plan: self.plan,
            state: Frozen {
                completion,
                turns,
                conversation: self.state.conversation,
            },
        })
    }

    pub fn turns_taken(&self) -> u32 {
        self.state.turns
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
