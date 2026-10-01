// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A dispatch whose resident is an official harness, on the accounting
//! thread: what the lane is handed, and what the city does with the run
//! once it comes home (sprawling-SPEC.md 8-124).
//!
//! The lane half - the tree, the harness, the turn - is
//! `driving::harness`. What is here writes on this thread: the run's
//! identity and its origin in the store before the lane, and after it
//! the request the harness cannot open itself, the tree given back and
//! what the city owed whoever asked.

use kernel::{AxError, Completion, TimeMs};

use super::super::driving::harness::{Chartered, HarnessDriven, HarnessHalf, harness_provenance};
use super::super::reviewing::Offering;
use super::super::workbench::tree_scope;
use super::super::{Landed, Owing, RunWorker};
use super::preparing::Staged;
use super::running::Continuation;
use super::{Assignment, Dispatched, Given, HarnessSeat, run_id_for};

impl RunWorker {
    /// Everything the accounting thread decides for a harness run once
    /// the room and the brief exist: who runs, under which id, and the
    /// half the lane drives.
    ///
    /// # Errors
    /// Propagates a resident description that cannot be read, a clock
    /// this machine will not read, a store that will not take the
    /// brief's origin, a backlog that will not take the run, and a
    /// handoff the runtime refuses.
    pub(super) fn stage_harness(
        &mut self,
        at: Assignment,
        seat: HarnessSeat,
        given: Given,
        began: TimeMs,
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
        // The run begins the room's session. What a branched session
        // carried is spent here and not handed on: ACP opens a session
        // with no history (sprawling-SPEC.md 8-124).
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
            format!("{} took this turn in the room's own tree", harness.as_str()),
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
        let spent = self.clock.now()?.value().saturating_sub(began.value());
        self.note(
            runtime::diagnostics::Level::Trace,
            "accounting::worker",
            &format!("prepare_dispatch took {spent} ms for {}", at.addr.as_str()),
        );
        Ok((Staged::Harness { at, half }, Continuation::Harness))
    }

    /// Puts a harness run's landing on the history: the backlog place
    /// and the tree go back whatever happened, and whoever asked is told
    /// how it ended.
    ///
    /// # Errors
    /// Propagates the drive's own failure, a backlog that will not take
    /// the member back, a tree that will not go back, and what the city
    /// owed a parent that will not take it.
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
            member,
            lease,
            outcome,
        } = driven;
        // Both loans go back before either failure is propagated, for the
        // reason a model run's landing gives (sprawling-SPEC.md 8-46-9);
        // a run frozen done first offers the tree it worked in.
        let left = member.map_or(Ok(()), |id| self.flight.backlog.leave(id));
        let offered = match (&outcome, &lease) {
            (Ok(Completion::Done(_)), Some(tree)) => self.offer_for_harness(
                &Offering {
                    tree: tree.path(),
                    scopes: &tree_scope(&building),
                    of: &harness_provenance(city, run, &at.addr, at.predecessor()),
                    who: &who,
                    addr: &at.addr,
                    run_id: run,
                },
                tree.name().as_str(),
            ),
            (Ok(_) | Err(_), _) => Ok(()),
        };
        let returned = self.give_tree_back(lease);
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
    /// Opens the request a harness run cannot open itself: harness
    /// residents reach none of the city's tools (sprawling-SPEC.md 8-4e
    /// rule 7). A branch that already has a request waiting keeps it,
    /// and the verifier judges the branch as it now stands.
    ///
    /// # Errors
    /// Propagates [`RunWorker::offer`]'s refusals.
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

/// A minute on the city's clock.
const MINUTE_MS: u64 = 60_000;

#[cfg(test)]
mod tests;
