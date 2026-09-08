// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run is frozen with: its plan, and the handoff that resumes it.

use std::path::Path;

use kernel::Locator;
use kernel::{Address, AxCode, AxError};
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::RunPlan;

use super::{
    Assignment, DISPATCH_TURN_BUDGET, Given, RunWorker, Site, Workbench, city_segment, name_of,
};

/// One line ending, named once. The prefix joins documents with it, and
/// a literal `10` at four call sites is four chances to mean something
/// else.
pub(super) const NEWLINE: u8 = 10;

/// The building slot: where this run stands, then the rules it stands
/// under.
///
/// `BUILDING.md` is here rather than left for the agent to open because
/// it is exactly as stable as the resident's own file — a person writes
/// it, no run may write it, and it does not move for the length of a
/// session. A rule an agent has to fetch before it can obey it is a rule
/// that gets obeyed one turn late, or not at all.
pub(super) fn building_segment(city_root: &Path, addr: &Address, building: &Address) -> Vec<u8> {
    let mut out = addr.as_str().as_bytes().to_vec();
    if let Ok(rules) = std::fs::read(city::building_path(city_root, building)) {
        out.push(NEWLINE);
        out.push(NEWLINE);
        out.extend_from_slice(&rules);
    }
    out
}

/// The run slot: what the last session left behind, then what this one
/// was asked for.
///
/// In that order, because the brief is what the agent acts on and the
/// last thing in a prompt is the thing that is read. A handoff that is
/// still its blank form contributes nothing and is left out.
/// # Errors
/// Propagates a handoff that exists and cannot be read: a prefix that
/// left it out would tell the next session there was none.
pub(super) fn run_segment(
    city_root: &Path,
    building: &Address,
    brief: &city::RunBrief,
) -> Result<Vec<u8>, AxError> {
    let mut out = Vec::new();
    if let Some(handoff) = city::handoff(city_root, building)? {
        out.extend_from_slice(handoff.as_bytes());
        out.push(NEWLINE);
        out.push(NEWLINE);
    }
    out.extend_from_slice(brief.segment_text().as_bytes());
    Ok(out)
}

pub(super) fn task_line(plan: &RunPlan) -> String {
    format!("{} at {}", plan.task, plan.addr.as_str())
}

impl RunWorker {
    /// Puts one desk's effects on the ledger, and only then makes the
    /// change they announce.
    ///
    /// This is the one door for all five of them. The change arrives as
    /// the return value of `Landing::record`, which appends first, so
    /// there is no expression in this file that reaches the city before
    /// the history - and the match below is exhaustive, so a new kind of
    /// change has to say here what it is.
    ///
    /// # Errors
    /// Propagates the first line the ledger refuses, in which case
    /// nothing changes; then whatever delivering, writing the plan or
    /// filing an entry reports.
    /// Freezes what this run is: the plan it drives on, and the handoff
    /// that says what to read to pick it up again.
    ///
    /// One phase because the two are one decision. The prefix is
    /// assembled for this plan and frozen with it, the handoff quotes
    /// the plan's own task line, and the job locator ends up in both -
    /// pinned in the store, so what a resumed run reads is the bytes the
    /// run segment carried rather than a file somebody edited since.
    ///
    /// # Errors
    /// Propagates a city or run segment that will not read, a norm on
    /// the must-read list that will not open, a store that will not take
    /// the bytes, and a handoff the runtime refuses.
    pub(super) fn freeze_plan(
        &mut self,
        site: &Site,
        workbench: &Workbench,
        at: &Assignment,
        given: Given,
    ) -> Result<(RunPlan, runtime::handoff::Handoff), AxError> {
        let addr = &at.addr;
        let Given {
            brief,
            task,
            goal,
            job,
        } = given;
        let tools = workbench.catalog.borrow().tool_defs();
        // The catalog is part of the resident segment, not a fifth slot:
        // what a resident may reach is as much a standing fact about it
        // as who it is, and both are frozen for the whole run so the
        // prefix stays cacheable across the run's life. Assembled here
        // rather than earlier because the catalog does not exist until
        // the tools, the reading room and the mode are known.
        // The name a person typed when they started this session, which
        // is the last segment of the address they started it at. It
        // opens the resident slot rather than the city one: the city
        // segment is identical for every agent in the city and is
        // cached as such, and a name in it would make one copy per
        // agent of the largest stable block in the prompt.
        let mut resident = format!("Your name: {}\n\n", name_of(addr)).into_bytes();
        resident.extend_from_slice(&site.identity.segment_bytes());
        resident.push(NEWLINE);
        resident.extend_from_slice(workbench.catalog.borrow().render().as_bytes());
        let prefix = FrozenPrefix::assemble(
            FrozenSegment::new(SegmentSlot::City, city_segment(&self.city_root)?),
            FrozenSegment::new(
                SegmentSlot::Building,
                building_segment(&self.city_root, addr, site.building.addr()),
            ),
            FrozenSegment::new(SegmentSlot::Resident, resident),
            FrozenSegment::new(
                SegmentSlot::Run,
                run_segment(&self.city_root, site.building.addr(), &brief)?,
            ),
        )?;

        let plan = RunPlan {
            run: site.run_id,
            who: site.who.clone(),
            addr: addr.clone(),
            task,
            goal,
            // The city decided this when it laid the brief down; the
            // window and the run segment read the one decision.
            opening: match &brief {
                city::RunBrief::Job { .. } => runtime::Opening::FromJob,
                city::RunBrief::Principal => runtime::Opening::WithPerson,
            },
            job: job.clone(),
            parent: at.parent,
            budget_turns: DISPATCH_TURN_BUDGET,
            budget: at.budget,
            shape: runtime::turn::CallShape {
                model: site.model.id.clone(),
                // The model's own ceiling, not a number chosen here.
                // With thinking enabled this budget covers reasoning and
                // answer together, so a hand-picked value truncates runs
                // for a reason that appears nowhere in the account.
                max_tokens: site.model.max_output_tokens,
                // Stated in CONFIG.toml, resolved down the three-layer
                // ladder, and frozen with the run.
                effort: site.config.effort,
            },
            prefix,
            policy: site.rules.policy().clone(),
            tools,
            // From the catalog rather than from a second scan of the
            // shelves: the catalog already decided what this run can
            // reach, and reading the shelf again would answer that
            // question a second time at a different instant.
            skills: workbench.catalog.borrow().skill_pins(),
        };

        // The norms are filled by the machine: their addresses are known
        // when the building is laid out, and a model asked to recite the
        // list from memory gets one entry wrong eventually.
        let mut must_read = Vec::new();
        for norm in city::norms(&self.city_root, addr)? {
            let bytes = std::fs::read(&norm).map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "read a norm document for the must-read list",
                    format!("{}: {err}", norm.display()),
                )
                .with_recovery("fix the file's permissions, or remove it from the building")
            })?;
            let hash = self.cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
            must_read.push(Locator::parse(&format!("cas:b3-{hash}"))?);
        }
        must_read.push(job);
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            task_line(&plan),
            "see the city roadmap".to_owned(),
            "dispatched from the control surface".to_owned(),
            "resume from the job locator".to_owned(),
        )?;
        Ok((plan, handoff))
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
