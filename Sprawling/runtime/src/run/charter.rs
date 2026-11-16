// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run's opening pair and its closing pair, written once for every
//! kind of run (`crates/runtime/spec/Run.lean` §8-52, runtime D7).
//!
//! A model run and a harness run open and close alike: the job pin, then
//! `run_started`; `handoff_written`, then `run_frozen` a millisecond
//! later. What differs is what happens between, and only a model run has
//! a `RunPlan` to say it, so the pairs are written from a view both kinds
//! of run can lend.

use kernel::event::Who;
use kernel::event::record::{CheckpointCommitted, Opening, RunFrozen, RunStarted};
use kernel::{
    Address, AxCode, AxError, Completion, EventDraft, EventKind, Ledger, Locator, Payload, RunId,
    TimeMs,
};

use crate::catalog::SkillPin;
use crate::handoff::Handoff;

use super::RunPlan;

/// How a run ended, with the error that ended it when an error did.
///
/// One value because the two are written into one `run_frozen` line, and
/// because only a cancelled ending carries a failure: a `done` verdict
/// with an error beside it is not a line a run can write.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Verdict<'a> {
    pub(crate) completion: &'a Completion,
    pub(crate) failure: Option<&'a AxError>,
}

/// What `run_started` records about a run, and whom every line of the
/// run is written as.
#[derive(Debug, Clone, Copy)]
pub struct Charter<'a> {
    pub run: RunId,
    /// The resident the run's lines are written as.
    pub who: &'a str,
    pub addr: &'a Address,
    pub task: &'a str,
    pub goal: &'a str,
    pub job: &'a Locator,
    pub parent: Option<RunId>,
    pub predecessor: Option<RunId>,
    pub dispatched_by: &'a Who,
    pub skills: &'a [SkillPin],
    /// The run policy `run_started` records (`crates/kernel/Spec.lean` §8-77).
    pub policy: kernel::RunPolicy,
    /// The identity version `run_started` records (`crates/kernel/Spec.lean` §8-79).
    pub naming: Option<kernel::B3Hash>,
    /// How the run's first user message was written, which a fork reads
    /// back (`crates/kernel/Spec.lean` §8-82-1); `None` for a run the city writes no
    /// first message for.
    pub opening: Option<Opening>,
    /// The effort `run_started` records (`crates/kernel/Spec.lean` §8-85):
    /// the one the run's requests froze; `None` when none was stated, and
    /// for a run that sends this city no requests.
    pub effort: Option<kernel::Effort>,
    /// The effective generation `run_started` records for a model run.
    pub generation: Option<&'a kernel::FrozenGeneration>,
    /// The native model routing this run recorded; `None` for a harness.
    pub routing: Option<&'a kernel::event::record::ModelRouting>,
    /// The ACP agent `run_started` records for a harness run
    /// (`crates/kernel/spec/Event/Record.lean` §8-87); `None` for a run this
    /// city's own model loop drives.
    pub agent: Option<&'a kernel::event::record::AgentRunIdentity>,
}

impl RunPlan {
    /// The charter a model run opens and closes with, lent from its own
    /// constants.
    pub fn charter(&self) -> Charter<'_> {
        Charter {
            run: self.run,
            who: &self.who,
            addr: &self.addr,
            task: &self.task,
            goal: &self.goal,
            job: &self.job,
            parent: self.parent,
            predecessor: self.predecessor,
            dispatched_by: &self.dispatched_by,
            skills: &self.skills,
            policy: self.run_policy.reader().now(),
            naming: self.naming,
            opening: Some(self.opening),
            effort: self.shape.effort,
            generation: Some(&self.shape.generation),
            routing: self.routing.as_ref(),
            agent: None,
        }
    }
}

impl Charter<'_> {
    /// The dispatch pair: the job pin lands first, then the run exists.
    /// Two lines, two clock samples - the pin is a fact about the city
    /// and the start is a fact about the run.
    ///
    /// # Errors
    /// Propagates the clock and the ledger.
    pub(crate) fn open(
        &self,
        ledger: &mut dyn Ledger,
        now: &mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Result<(), AxError> {
        let pin = CheckpointCommitted::JobPinned {
            job: self.job.clone(),
        };
        ledger.append(EventDraft {
            run: RunId::CITY,
            t: now()?,
            who: Who::City.to_string(),
            addr: Some(self.addr.clone()),
            kind: EventKind::CheckpointCommitted,
            data: Payload::of(&pin)?,
            ig: false,
        })?;
        let started = RunStarted {
            task: self.task.to_owned(),
            goal: self.goal.to_owned(),
            job: Some(self.job.clone()),
            parent: self.parent,
            predecessor: self.predecessor,
            skills: self.skills.to_vec(),
            dispatched_by: Some(self.dispatched_by.clone()),
            policy: Some(self.policy),
            naming: self.naming,
            opening: self.opening,
            effort: self.effort,
            generation: self.generation.cloned(),
            routing: self.routing.cloned(),
            agent: self.agent.cloned(),
        };
        ledger.append(EventDraft {
            run: self.run,
            t: now()?,
            who: Who::City.to_string(),
            addr: Some(self.addr.clone()),
            kind: EventKind::RunStarted,
            data: Payload::of(&started)?,
            ig: false,
        })?;
        Ok(())
    }

    /// The closing pair: the handoff at `t`, and `run_frozen` one
    /// millisecond later, because the two lines record one event. The
    /// verdict says how the run ended and what stopped it, when an error
    /// did.
    ///
    /// # Errors
    /// Propagates the ledger, and refuses a `t` at the end of `u64`.
    pub(crate) fn close(
        &self,
        ledger: &mut dyn Ledger,
        handoff: &Handoff,
        verdict: Verdict<'_>,
        t: TimeMs,
    ) -> Result<(), AxError> {
        ledger.append(EventDraft {
            run: self.run,
            t,
            who: self.who.to_owned(),
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
            run: self.run,
            t: TimeMs::new(closing),
            who: self.who.to_owned(),
            addr: None,
            kind: EventKind::RunFrozen,
            data: Payload::of(&RunFrozen::of_failure(verdict.completion, verdict.failure))?,
            ig: false,
        })?;
        Ok(())
    }

    /// One line of this run, written as its resident at its address.
    pub(crate) fn line(&self, kind: EventKind, data: Payload, t: TimeMs) -> EventDraft {
        EventDraft {
            run: self.run,
            t,
            who: self.who.to_owned(),
            addr: Some(self.addr.clone()),
            kind,
            data,
            ig: false,
        }
    }
}
