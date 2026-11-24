// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Who;
use kernel::event::record::{CheckpointCommitted, Opening, RunFrozen, RunStarted};
use kernel::{
    Address, AxCode, AxError, Completion, EventDraft, EventKind, Ledger, Locator, Payload, RunId,
    TimeMs,
};

use crate::catalog::SkillPin;
use crate::handoff::Handoff;

use super::RunPlan;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Verdict<'a> {
    pub(crate) completion: &'a Completion,
    pub(crate) failure: Option<&'a AxError>,
}

#[derive(Debug, Clone, Copy)]
pub struct Charter<'a> {
    pub pictures: &'a [kernel::ImageRef],
    pub run: RunId,

    pub who: &'a str,
    pub addr: &'a Address,
    pub task: &'a str,
    pub goal: &'a str,
    pub job: &'a Locator,
    pub parent: Option<RunId>,
    pub predecessor: Option<RunId>,
    pub dispatched_by: &'a Who,
    pub skills: &'a [SkillPin],

    pub policy: kernel::RunPolicy,

    pub naming: Option<kernel::B3Hash>,

    pub opening: Option<Opening>,

    pub effort: Option<kernel::Effort>,

    pub generation: Option<&'a kernel::FrozenGeneration>,

    pub routing: Option<&'a kernel::event::record::ModelRouting>,

    pub agent: Option<&'a kernel::event::record::AgentRunIdentity>,
}

impl RunPlan {
    pub fn charter(&self) -> Charter<'_> {
        Charter {
            pictures: &self.pictures,
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
            pictures: self.pictures.to_vec(),
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
