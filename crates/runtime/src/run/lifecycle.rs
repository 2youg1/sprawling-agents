// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an active run does on the ledger: the dispatch pair that brings
//! it into existence, one turn, and the freeze that is its only exit.

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

use super::checkpoint::{CheckpointPolicy, Wave, WaveCheckpoint};
use super::{Active, Advance, Frozen, Run, RunHooks, RunPlan, SafePoint};

/// A steer changes what the model reads next, so the driver folds it into
/// the window it owns; the turn layer records that it arrived. Two halves
/// of one event, each held where its material is: the text belongs to the
/// window, the record belongs to the ledger.
///
/// Whichever safe point it arrives at, the fold takes effect at the next
/// assembly — a steer never rewrites a request already on the wire, which
/// the conversation holds by knowing what the last assembly sent.
fn fold_steer(conversation: &mut Conversation, interrupt: &Interrupt) {
    if let Interrupt::Steer { speaker, text } = interrupt {
        conversation.push_steer(speaker, text);
    }
}

/// A policy change may reach the run at any safe point; it waits in the
/// cell until the next `BeforeWave` (`crates/runtime/spec/PolicyTake.lean`
/// §8-62).
fn hold_policy(cell: &mut PolicyCell, interrupt: &Interrupt) {
    if let Interrupt::Policy { policy } = interrupt {
        cell.arrive(*policy);
    }
}

/// What a safe point hands the driver: a steer joins the window, a policy
/// change waits in the cell. One door for both, so no safe point can
/// forget either.
fn fold_arrival(conversation: &mut Conversation, policy: &mut PolicyCell, interrupt: &Interrupt) {
    fold_steer(conversation, interrupt);
    hold_policy(policy, interrupt);
}

/// How a turn that called no tool ends the run.
///
/// **A reply with nothing in it is not evidence that work finished.**
/// Two replies arrive that way: one the provider cut off at the model's
/// output ceiling, and one that carries no content at all — which is
/// what a request stating a ceiling of zero earns from a provider that
/// accepts the number. Freezing either as done tells a person their work
/// is finished at the moment nothing was said, and cites a
/// `model_returned` holding no content as the evidence for it. Both are
/// `Limit`: the run ended against something rather than at the end of
/// its work, and the account says which.
///
/// `lines` must already have carried the report's `model_returned` to
/// the ledger, because the evidence cites its ref.
fn concluded(report: &TurnReport, lines: &HeldLines) -> Result<Completion, AxError> {
    if report.assistant().is_empty() || report.stop() == Some(StopReason::MaxTokens) {
        return Ok(Completion::Limit);
    }
    Ok(Completion::Done(Evidence::new(vec![
        lines.durable(report.model_returned())?,
    ])?))
}

impl Run<Active> {
    /// One line of this run's, written by the driver at `t`.
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

    /// The `watchdog_fired` line for a disposal that sends the call
    /// again: written before the wait or the switch, so the next
    /// `model_called` in the history follows the reason for it.
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

    /// A refusal the driver itself raised, written under its code's
    /// carrier event.
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

    /// The dispatch pair: the job pin lands first, then the run exists.
    /// Two ledger lines, two clock samples — the pin is a fact about the
    /// city and the start is a fact about the run. Written by the plan's
    /// charter, the one author of every run's opening (`crates/runtime/spec/Run.lean`
    /// §8-52).
    ///
    /// # Errors
    /// Propagates whatever the ledger says; nothing else here can fail.
    pub fn dispatch(
        plan: RunPlan,
        ledger: &mut dyn Ledger,
        hooks: &mut RunHooks<'_>,
    ) -> Result<Run<Active>, AxError> {
        plan.charter().open(ledger, &mut *hooks.now)?;
        let lines = HeldLines::open(plan.run, plan.who.clone());

        let mut conversation = Conversation::new();
        // A branch opens with the conversation it branched from, and then
        // its own lines: the task joins the mother's last user message when
        // one is open, which is the same rule a steer follows.
        conversation.push_inherited(&plan.inherited);
        conversation.push_task_lines(
            &plan.task,
            &plan.goal,
            plan.opening,
            &plan
                .dispatched_by
                .handed_down_by(plan.predecessor, plan.parent),
        );
        let gauge = ContextGauge::new(
            kernel::Tokens::new(plan.shape.context_tokens),
            plan.second_threshold,
        );
        let watchdog = crate::Watchdog::new(plan.retries, plan.run);
        Ok(Run {
            plan,
            state: Active {
                conversation,
                turns: 0,
                last_turn_t: None,
                gauge,
                prior_shape: None,
                prompt: crate::turn::PromptRecord::default(),
                checkpoint: CheckpointPolicy::opening(),
                lines,
                watchdog,
            },
        })
    }

    /// Takes one turn. Every exit from a phase is either the next phase or
    /// a cancellation, so the three safe points are the only places an
    /// interruption can land.
    ///
    /// # Errors
    /// Propagates ledger and provider failures. A tool failure is not one
    /// of them: it reaches the model as this call's result.
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
        // What this request looks like to a prompt cache, and which of its
        // regions moved since the request before it. Written here, after the
        // turn has assembled, so the line describes a request that exists;
        // the shape of the first request says so rather than claiming a
        // comparison nobody made. It is held among the turn's lines, so it
        // goes down with `model_called` rather than through a barrier of its
        // own.
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
            // Reborrowed rather than moved: the sink and the tool face
            // belong to the hooks and every later turn needs them too.
            Generating::Speculating {
                deltas: hooks.deltas.as_deref_mut(),
                tools: &*hooks.invoke,
            },
        );
        // The request borrowed the conversation until the call returned,
        // so the conversation moves only now: what was assembled is on
        // the wire, and a steer that arrived before the call joins the
        // next request, whatever the call answered.
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

        // Recorded before the wave, so a `status` call in it reports the
        // count of the very call that asked for it.
        if let Some(usage) = turn.usage() {
            self.plan.context.record(usage.input_tokens);
        }
        // The checkpoint goes up before the wave, not before a suspicious call:
        // anything the wave deletes then has a commit to come back from.
        let touches = Wave::of(turn.calls(), hooks.writes);
        let decided = self.state.checkpoint.for_wave(touches);
        if let (WaveCheckpoint::Stage, Some(checkpoint)) = (decided, hooks.checkpoint.as_mut()) {
            turn.hold_run_line(
                RunLine::CheckpointCommitted,
                self.plan.addr.clone(),
                checkpoint(t)?,
            );
        }

        self.state.checkpoint.record_wave(decided, touches);

        let wave = (hooks.interrupt)(SafePoint::BeforeWave { turn: index });
        fold_arrival(
            &mut self.state.conversation,
            &mut self.plan.run_policy,
            &wave,
        );
        // The one place the policy in force changes: every call of the
        // wave below is judged under what the cell holds now.
        let taken = self.plan.run_policy.take_at_wave();
        // The same question the three phase boundaries ask, asked again
        // before each call of the wave. A cancel ends the wave there; a
        // steer is recorded by the turn and folded into the window, and
        // the call goes ahead, because text that redirects the work takes
        // effect at the next assembly and stopping is the only
        // instruction that can be carried out between two calls.
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
        self.state.turns = self.state.turns.saturating_add(1);
        self.state
            .conversation
            .push_assistant(report.assistant().to_vec());
        self.state
            .conversation
            .push_tool_results(report.wave_results().to_vec());
        // The model learns of a change after the results of the first wave
        // judged under it, at the end of the window, so the bytes already
        // sent stay a prefix of the next request (§8-62).
        if let Some(policy) = taken {
            self.state
                .conversation
                .push_steer(&crate::conversation::Speaker::City, &policy_note(&policy));
        }
        // The provider's count for this call, against the model's
        // window: a fact against a fact. It lands after the results the
        // model reads next, by the door a steer takes.
        if let Some(reminder) = report
            .usage()
            .and_then(|usage| self.state.gauge.observe(usage.input_tokens))
        {
            self.state.conversation.push_reminder(&reminder);
        }
        if report.calls_made() == 0 {
            self.state.lines.barrier(ledger)?;
            return Ok(Advance::Concluded(concluded(&report, &self.state.lines)?));
        }
        Ok(Advance::Turned)
    }

    /// The only exit. The run's held lines go down first, in the one
    /// barrier a run pays at its end (runtime D36); then both lines are
    /// written here, so no caller can end a
    /// run by simply dropping it and leaving the ledger without a verdict.
    ///
    /// A cancelled run freezes inside the turn it interrupted and carries
    /// that turn's stamp; any other ending samples the clock once, and
    /// `run_frozen` follows one millisecond later because the two lines
    /// record one event.
    ///
    /// # Errors
    /// Propagates ledger failures and a clock that has run past `u64`.
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
        self.plan.charter().close(ledger, handoff, &completion, t)?;
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
