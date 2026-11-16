// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, Locator, RunId};

pub(super) struct Assignment {
    pub(super) addr: Address,

    pub(super) session: Option<kernel::SessionName>,

    pub(super) effort: Option<kernel::Effort>,

    pub(super) difficulty: Option<kernel::Difficulty>,

    pub(super) model: Option<String>,

    pub(super) policy: kernel::RunPolicy,

    pub(super) parent: Option<RunId>,

    pub(super) succession: Option<Handover>,

    pub(super) taint: kernel::TaintSet,

    pub(super) origin: Option<kernel::Origin>,

    pub(super) dispatched_by: kernel::event::Who,
}

pub(super) struct Handover {
    pub(super) predecessor: RunId,
    pub(super) before: super::probing::probe::Answers,
}

impl Assignment {
    pub(super) fn predecessor(&self) -> Option<RunId> {
        self.succession.as_ref().map(|handed| handed.predecessor)
    }

    pub(super) fn depth(&self) -> kernel::Depth {
        match self.parent {
            None => kernel::Depth::Root,
            Some(_) => kernel::Depth::Delegated,
        }
    }
}

pub(super) struct Given {
    pub(super) brief: city::RunBrief,
    pub(super) task: String,
    pub(super) goal: String,
    pub(super) job: Locator,
}

impl super::RunWorker {
    pub(super) fn give(
        &mut self,
        at: &Assignment,
        task: String,
        goal: String,
    ) -> Result<Given, AxError> {
        let brief = city::write_brief(
            &self.city_root,
            &at.addr,
            &city::JobBrief {
                from: &at.dispatched_by.handed_down_by(at.predecessor(), at.parent),
                task: &task,
                goal: &goal,
            },
        )?;

        let job_hash = self
            .cas
            .put(brief.segment_text().as_bytes())
            .map_err(storage::StorageError::into_ax)?;
        Ok(Given {
            job: Locator::cas(job_hash),
            brief,
            task,
            goal,
        })
    }
}

pub(super) struct Agreed {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    pub(super) model: gateway::ModelEntry,

    pub(super) routing: kernel::event::record::ModelRouting,

    pub(super) provider: String,
    pub(super) adapter: super::keeping_warm::Door,

    pub(super) retries: kernel::Retries,
}

pub(super) enum Seat {
    Model(Agreed),

    Harness(HarnessSeat),
}

pub(super) struct HarnessSeat {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    pub(super) harness: agent_protocols::Consented,
}

impl Seat {
    pub(super) fn building(&self) -> &city::Building {
        match self {
            Seat::Model(agreed) => &agreed.building,
            Seat::Harness(seat) => &seat.building,
        }
    }
}

pub(super) struct Knock {
    pub(super) addr: Address,

    pub(super) from: String,

    pub(super) policy: kernel::RunPolicy,

    pub(super) chain: super::KnockChain,
}

pub(crate) struct Dispatched {
    pub(super) run: RunId,
    pub(super) addr: Address,
    pub(super) who: String,
    pub(super) completion: kernel::Completion,
}

pub(super) mod agreeing;
pub(super) mod custody;
pub(super) mod handback;
pub(super) mod harness;
pub(super) mod heavy_landing;
pub(super) mod preparing;
pub(super) mod routing;
pub(super) mod running;
pub(super) mod session;
pub(super) mod session_shape;
pub use agreeing::acp_dispatch;
pub(super) use agreeing::run_id_for;
#[cfg(test)]
mod tests;
