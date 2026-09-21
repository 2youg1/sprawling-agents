// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an active run does on the ledger: the dispatch pair that brings
//! it into existence, one turn, and the freeze that is its only exit.

use kernel::event::Who;
use kernel::event::record::{CheckpointCommitted, RunFrozen, RunStarted};
use kernel::{
    AxCode, AxError, Completion, EventDraft, EventKind, Evidence, Ledger, Model, Payload, RunId,
    StopReason, TimeMs, ToolCall,
};

use crate::handoff::Handoff;
use crate::reminder::ContextGauge;
use crate::turn::{Interrupt, NextCall, PhaseOutcome, Turn, TurnReport};
use crate::window::Window;

use super::{Active, Advance, Frozen, Run, RunHooks, RunPlan, SafePoint};

/// A steer changes what the model reads next, so the driver folds it into
/// the window it owns; the turn layer records that it arrived. Two halves
/// of one event, each held where its material is: the text belongs to the
/// window, the record belongs to the ledger.
///
/// Whichever safe point it arrives at, the fold takes effect at the next
/// assembly — a steer never rewrites a request already on the wire.
fn fold_steer(window: &mut Window, interrupt: &Interrupt) {
    if let Interrupt::Steer { source, text } = interrupt {
        window.push_steer(source, text);
    }
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
fn concluded(report: &TurnReport) -> Result<Completion, AxError> {
    if report.assistant().is_empty() || report.stop() == Some(StopReason::MaxTokens) {
        return Ok(Completion::Limit);
    }
    Ok(Completion::Done(Evidence::new(vec![
        *report.model_returned(),
    ])?))
}

impl Run<Active> {
    /// The dispatch pair: the job pin lands first, then the run exists.
    /// Two ledger lines, two clock samples — the pin is a fact about the
    /// city and the start is a fact about the run.
    ///
    /// # Errors
    /// Propagates whatever the ledger says; nothing else here can fail.
    pub fn dispatch(
        plan: RunPlan,
        ledger: &mut dyn Ledger,
        hooks: &mut RunHooks<'_>,
    ) -> Result<Run<Active>, AxError> {
        let pin_t = (hooks.now)()?;
        let pin = CheckpointCommitted::JobPinned {
            job: plan.job.clone(),
        };
        ledger.append(EventDraft {
            run: RunId::CITY,
            t: pin_t,
            who: Who::City.to_string(),
            addr: Some(plan.addr.clone()),
            kind: EventKind::CheckpointCommitted,
            data: Payload::of(&pin)?,
            ig: false,
        })?;

        let start_t = (hooks.now)()?;
        let started = RunStarted {
            task: plan.task.clone(),
            goal: plan.goal.clone(),
            job: Some(plan.job.clone()),
            parent: plan.parent,
            predecessor: plan.predecessor,
            skills: plan.skills.clone(),
        };
        ledger.append(EventDraft {
            run: plan.run,
            t: start_t,
            who: Who::City.to_string(),
            addr: Some(plan.addr.clone()),
            kind: EventKind::RunStarted,
            data: Payload::of(&started)?,
            ig: false,
        })?;

        let mut window = Window::new();
        window.push_task_lines(&plan.task, &plan.goal, plan.opening);
        let gauge = ContextGauge::new(kernel::Tokens::new(plan.shape.context_tokens));
        Ok(Run {
            plan,
            state: Active {
                window,
                turns: 0,
                last_turn_t: None,
                gauge,
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

        let turn = Turn::begin(self.plan.run, self.plan.who.clone(), t);
        let opening = (hooks.interrupt)(SafePoint::BeforeAssemble { turn: index });
        fold_steer(&mut self.state.window, &opening);
        let turn = match turn.assemble(
            opening,
            ledger,
            &self.plan.prefix,
            &self.state.window,
            &self.plan.tools,
            &self.plan.shape,
        )? {
            PhaseOutcome::Advanced(next) => next,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };
        let calling = (hooks.interrupt)(SafePoint::BeforeCall { turn: index });
        fold_steer(&mut self.state.window, &calling);
        let turn = match turn.call(
            calling,
            ledger,
            model,
            &self.plan.policy,
            // Reborrowed rather than moved: the sink belongs to the
            // hooks and every later turn needs it too.
            hooks.deltas.as_deref_mut(),
        )? {
            PhaseOutcome::Advanced(next) => next,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };

        // The fence goes up before the wave, not before a suspicious call:
        // anything the wave deletes then has a commit to come back from.
        if let Some(fence) = hooks.fence.as_mut() {
            let committed = fence(t)?;
            ledger.append(EventDraft {
                run: self.plan.run,
                t,
                who: self.plan.who.clone(),
                addr: Some(self.plan.addr.clone()),
                kind: EventKind::CheckpointCommitted,
                data: committed,
                ig: false,
            })?;
        }

        let wave = (hooks.interrupt)(SafePoint::BeforeWave { turn: index });
        fold_steer(&mut self.state.window, &wave);
        let invoke = &mut hooks.invoke;
        let mut stamped = |call: &ToolCall| invoke(call, t);
        // The same question the three phase boundaries ask, asked again
        // before each call of the wave. A cancel ends the wave there; a
        // steer is folded into the window and the call goes ahead,
        // because text that redirects the work takes effect at the next
        // assembly and stopping is the only instruction that can be
        // carried out between two calls.
        let asking = &mut hooks.interrupt;
        let window = &mut self.state.window;
        let mut still_going = |call: u32| {
            let arrived = asking(SafePoint::BeforeToolCall { turn: index, call });
            fold_steer(window, &arrived);
            match arrived {
                Interrupt::Cancel => NextCall::Halted,
                Interrupt::None | Interrupt::Steer { .. } => NextCall::Allowed,
            }
        };
        let turn = match turn.execute(wave, ledger, &mut stamped, &mut still_going)? {
            PhaseOutcome::Advanced(next) => next,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };

        let settling = (hooks.interrupt)(SafePoint::BeforeSpawn { turn: index });
        fold_steer(&mut self.state.window, &settling);
        let report = match turn.record(settling, ledger)? {
            PhaseOutcome::Advanced(report) => report,
            PhaseOutcome::Cancelled(_) => return Ok(Advance::Concluded(Completion::Cancelled)),
        };
        self.state.turns = self.state.turns.saturating_add(1);
        self.state
            .window
            .push_assistant(report.assistant().to_vec());
        self.state
            .window
            .push_tool_results(report.wave_results().to_vec());
        // The provider's count for this call, against the model's
        // window: a fact against a fact. It lands after the results the
        // model reads next, by the door a steer takes.
        if let Some(reminder) = report
            .usage()
            .and_then(|usage| self.state.gauge.observe(usage.input_tokens))
        {
            self.state.window.push_reminder(&reminder);
        }
        if report.calls_made() == 0 {
            return Ok(Advance::Concluded(concluded(&report)?));
        }
        Ok(Advance::Turned)
    }

    /// The only exit. Both lines are written here, so no caller can end a
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
        let t = match (&completion, self.state.last_turn_t) {
            (Completion::Cancelled, Some(turn_t)) => turn_t,
            _ => (hooks.now)()?,
        };
        ledger.append(EventDraft {
            run: self.plan.run,
            t,
            who: self.plan.who.clone(),
            addr: None,
            kind: EventKind::HandoffWritten,
            data: handoff.payload()?,
            ig: false,
        })?;
        let closing = t.value().checked_add(1).ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "stamp run_frozen", "u64 overflow")
                .with_recovery("check the clock the caller injected")
        })?;
        ledger.append(EventDraft {
            run: self.plan.run,
            t: TimeMs::new(closing),
            who: self.plan.who.clone(),
            addr: None,
            kind: EventKind::RunFrozen,
            data: Payload::of(&RunFrozen::of(&completion))?,
            ig: false,
        })?;
        let turns = self.state.turns;
        Ok(Run {
            plan: self.plan,
            state: Frozen {
                completion,
                turns,
                window: self.state.window,
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
