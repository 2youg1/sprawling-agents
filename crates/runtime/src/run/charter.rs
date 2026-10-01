// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A run's opening pair and its closing pair, written once for every
//! kind of run (runtime-SPEC.md 8-52, 12.7).
//!
//! A model run and a harness run open and close alike: the job pin, then
//! `run_started`; `handoff_written`, then `run_frozen` a millisecond
//! later. What differs is what happens between, and only a model run has
//! a `RunPlan` to say it, so the pairs are written from a view both kinds
//! of run can lend.

use kernel::event::Who;
use kernel::event::record::{CheckpointCommitted, RunFrozen, RunStarted};
use kernel::{
    Address, AxCode, AxError, Completion, EventDraft, EventKind, Ledger, Locator, Payload, RunId,
    TimeMs,
};

use crate::catalog::SkillPin;
use crate::handoff::Handoff;

use super::RunPlan;

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
    /// The run policy `run_started` records (kernel-SPEC 8-77).
    pub policy: kernel::RunPolicy,
    /// The identity version `run_started` records (kernel-SPEC 8-79).
    pub naming: Option<kernel::B3Hash>,
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
            policy: self.run_policy,
            naming: self.naming,
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
    /// millisecond later, because the two lines record one event.
    ///
    /// # Errors
    /// Propagates the ledger, and refuses a `t` at the end of `u64`.
    pub(crate) fn close(
        &self,
        ledger: &mut dyn Ledger,
        handoff: &Handoff,
        completion: &Completion,
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
            data: Payload::of(&RunFrozen::of(completion))?,
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
