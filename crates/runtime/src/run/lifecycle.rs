// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an active run does on the ledger: the dispatch pair that brings
//! it into existence, one turn, and the freeze that is its only exit.

use kernel::{
    AxCode, AxError, Completion, EventDraft, EventKind, Evidence, Ledger, Model, RunId, TimeMs,
    ToolCall,
};
use serde_json::{Map, Value};

use crate::handoff::Handoff;
use crate::reminder::ContextGauge;
use crate::turn::{Interrupt, PhaseOutcome, Turn};
use crate::window::Window;

use super::{Active, Advance, Frozen, Run, RunHooks, RunPlan, SafePoint, payload};

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
        let mut pin = Map::new();
        pin.insert("job".to_owned(), Value::String(plan.job.to_string()));
        ledger.append(EventDraft {
            run: RunId::CITY,
            t: pin_t,
            who: "city".to_owned(),
            addr: Some(plan.addr.clone()),
            kind: EventKind::CheckpointCommitted,
            data: payload(pin)?,
            ig: false,
        })?;

        let start_t = (hooks.now)()?;
        let mut started = Map::new();
        started.insert("task".to_owned(), Value::String(plan.task.clone()));
        started.insert("goal".to_owned(), Value::String(plan.goal.clone()));
        started.insert("job".to_owned(), Value::String(plan.job.to_string()));
        if let Some(parent) = plan.parent {
            started.insert("parent".to_owned(), Value::String(parent.to_string()));
        }
        if let Some(predecessor) = plan.predecessor {
            started.insert(
                "predecessor".to_owned(),
                Value::String(predecessor.to_string()),
            );
        }
        // Unconditional rather than omitted when empty: a key that comes
        // and goes is a shape a reader has to guess at, and "this
        // building admits nothing" is a fact worth recording rather than
        // an absence to infer.
        started.insert(
            "skills".to_owned(),
            Value::Array(
                plan.skills
                    .iter()
                    .map(|pin| {
                        let mut row = Map::new();
                        row.insert("name".to_owned(), Value::String(pin.name.clone()));
                        row.insert("hash".to_owned(), Value::String(pin.hash.to_string()));
                        Value::Object(row)
                    })
                    .collect(),
            ),
        );
        ledger.append(EventDraft {
            run: plan.run,
            t: start_t,
            who: "city".to_owned(),
            addr: Some(plan.addr.clone()),
            kind: EventKind::RunStarted,
            data: payload(started)?,
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
        let turn = match turn.execute(wave, ledger, &mut stamped)? {
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
            let evidence = Evidence::new(vec![*report.model_returned()])?;
            return Ok(Advance::Concluded(Completion::Done(evidence)));
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
        let mut frozen = Map::new();
        completion.extend_payload(&mut frozen)?;
        ledger.append(EventDraft {
            run: self.plan.run,
            t: TimeMs::new(closing),
            who: self.plan.who.clone(),
            addr: None,
            kind: EventKind::RunFrozen,
            data: payload(frozen)?,
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
