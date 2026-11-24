// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Completion};

use super::super::driving::harness::{Chartered, HarnessDriven, HarnessHalf, harness_provenance};
use super::super::reviewing::Offering;
use super::super::workbench::tree_scope;
use super::super::{Landed, Owing, RunWorker};
use super::preparing::Staged;
use super::running::Continuation;
use super::{Assignment, Dispatched, Given, HarnessSeat, run_id_for};

impl RunWorker {
    pub(super) fn stage_harness(
        &mut self,
        at: Assignment,
        seat: HarnessSeat,
        given: Given,
        began: std::time::Instant,
    ) -> Result<(Staged, Continuation), AxError> {
        let HarnessSeat {
            building,
            rules,
            harness,
        } = seat;
        let who = city::Identity::load(&self.city_root, &at.addr)?.who();
        let run = run_id_for(&given.job, &at.addr, self.clock.now()?);
        self.cas
            .put_for(
                given.brief.segment_text().as_bytes(),
                &storage::BlockOrigin {
                    run,
                    building: at.addr.clone(),
                },
            )
            .map_err(storage::StorageError::into_ax)?;

        self.origins.started(&at.addr, run);
        let member = match at.parent {
            Some(_) => Some(
                self.flight
                    .backlog
                    .enrol_run(&at.addr, format!("run {run} at {}", at.addr.as_str()))?,
            ),
            None => None,
        };
        let prompt = match &given.brief {
            city::RunBrief::Job { text } => text.clone(),
            city::RunBrief::Principal => given.task.clone(),
        };
        let handoff = runtime::handoff::Handoff::new(
            vec![given.job.clone()],
            given.task.clone(),
            "what the harness did is the run's harness_reported lines".to_owned(),
            format!(
                "{} took this turn in the room's own tree",
                harness.entry().id.as_str()
            ),
            "read the request the city opened for the tree, or the tree itself".to_owned(),
        )?;
        let half = HarnessHalf {
            harness,
            start: std::sync::Arc::clone(&self.harnesses),
            chartered: Chartered {
                run,
                who,
                addr: at.addr.clone(),
                task: given.task,
                goal: given.goal,
                job: given.job,
                parent: at.parent,
                predecessor: at.predecessor(),
                dispatched_by: at.dispatched_by.clone(),
                policy: at.policy,
            },
            building,
            entropy_allow: rules.checkpoint_entropy_allow().clone(),
            prompt,
            handoff,
            city_root: self.city_root.clone(),
            city: self.city_hash()?,
            command: self.doorstep.entrance.carrying(),
            member,
            ceiling_ms: u64::from(rules.harness_minutes()).saturating_mul(MINUTE_MS),
            notes: self.log.clone(),
            staged_at: self.ledger.position(),
        };
        let spent = (self.monotonic)()
            .saturating_duration_since(began)
            .as_millis();
        self.note(
            runtime::diagnostics::Level::Trace,
            "accounting::worker",
            &format!("prepare_dispatch took {spent} ms for {}", at.addr.as_str()),
        );
        Ok((Staged::Harness { at, half }, Continuation::Harness))
    }

    pub(super) fn land_harness(
        &mut self,
        at: Assignment,
        driven: HarnessDriven,
        owing: Owing,
    ) -> Result<Landed, AxError> {
        let HarnessDriven {
            run,
            who,
            building,
            city,
            entropy_allow,
            member,
            lease,
            outcome,
        } = driven;

        let left = member.map_or(Ok(()), |id| self.flight.backlog.leave(id));
        let offered = match (&outcome, &lease) {
            (Ok(Completion::Done(_)), Some(tree)) => self.offer_for_harness(
                &Offering {
                    project: building.addr(),
                    tree: tree.path(),
                    scopes: &tree_scope(&building),
                    of: &harness_provenance(city, run, &at.addr, at.predecessor()),
                    who: &who,
                    addr: &at.addr,
                    run_id: run,
                    entropy_allow: &entropy_allow,
                },
                tree.name().as_str(),
            ),
            (Ok(_) | Err(_), _) => Ok(()),
        };
        let returned = if offered.is_ok() && outcome.is_ok() {
            self.retire_checkout(lease)
        } else {
            self.give_tree_back(lease)
        };
        left?;
        offered?;
        returned?;
        let completion = outcome?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "runtime::run",
            &format!("dispatch at {} finished on a harness", at.addr.as_str()),
        );
        let landed = self.discharge(
            owing,
            &at,
            &Dispatched {
                run,
                addr: at.addr.clone(),
                who,
                completion,
            },
        )?;
        self.answer_knocks();
        Ok(landed)
    }
}

impl RunWorker {
    fn offer_for_harness(&mut self, offering: &Offering<'_>, branch: &str) -> Result<(), AxError> {
        if self
            .collaborating
            .requests
            .iter()
            .any(|waiting| waiting.branch == branch)
        {
            self.note(
                runtime::diagnostics::Level::Effect,
                "collab::pr",
                &format!(
                    "{} already has a request waiting; {} added to it",
                    branch, offering.run_id
                ),
            );
            return Ok(());
        }
        self.offer(offering, branch.to_owned())
    }
}

const MINUTE_MS: u64 = 60_000;

#[cfg(test)]
mod tests;
