// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One building's plan: who holds what, and how far red reaches.

use kernel::{Address, AxCode, AxError, EventKind};
use kernel::{Payload, RunId};

use crate::effect;

use super::{Assignment, RunWorker, now_ms};

/// The run reporting a change to the plan: which run, of which
/// building, as whom. Three values that always travel together and are
/// never chosen independently, so they travel as one. The room is not
/// among them: it is the address the [`Assignment`] beside this one
/// already names, and two fields for one room are two chances to name
/// different ones.
#[derive(Clone, Copy)]
pub(super) struct Reporter<'a> {
    pub(super) run_id: RunId,
    pub(super) building: &'a Address,
    pub(super) who: &'a str,
}

impl RunWorker {
    /// Tells whoever is standing behind a node that has just gone red.
    ///
    /// **A fact starts this, not a person.** Every other way one
    /// resident reaches another in this city begins with somebody
    /// deciding to speak; this one begins with a node going red, and it
    /// reaches exactly the rooms holding work that cannot now move.
    ///
    /// The signal's id is derived from the building and the node, so a
    /// blockage announced twice is one signal: the inbox already
    /// deduplicates by id, and a room told four times about one problem
    /// is a room that stops reading its inbox.
    pub(super) fn tell_whoever_is_behind(
        &mut self,
        at: &Assignment,
        by: Reporter<'_>,
        effects: &[collab::ClaimEffect],
    ) -> Result<(), AxError> {
        let Reporter {
            run_id,
            building,
            who,
        } = by;
        let room = &at.addr;
        let red: Vec<kernel::RedNode> = effects
            .iter()
            .filter_map(|effect| match effect {
                collab::ClaimEffect::PutDown {
                    exit: kernel::PlanExit::Stopped { id, why },
                    ..
                } if why.is_red() => Some(kernel::RedNode {
                    at: id.clone(),
                    why: why.clone(),
                }),
                _ => None,
            })
            .collect();
        if red.is_empty() {
            return Ok(());
        }
        let Some(tree) = self.plan_of(building) else {
            return Ok(());
        };
        let held = self.holders_in(building);
        let mut sent = Vec::new();
        for notice in kernel::notices(&kernel::spread(&tree, &red), &held) {
            let Ok(to) = Address::parse(&notice.to) else {
                continue;
            };
            if &to == room {
                continue;
            }
            let mut payload = serde_json::Map::new();
            payload.insert(
                "text".to_owned(),
                serde_json::Value::String(notice.line.clone()),
            );
            sent.push(collab::SignalEffect::Enqueued(collab::Signal::new(
                collab::SignalId::parse(&format!(
                    "blocked-{}-{}",
                    building.as_str().replace('/', "-"),
                    notice.about
                ))?,
                collab::SignalKind::Mention,
                who.to_owned(),
                to,
                kernel::Version::FIRST,
                Payload::new(payload)?,
                now_ms()?,
            )?));
        }
        if sent.is_empty() {
            return Ok(());
        }
        let landing = effect::Landing::signals(sent, room, who)?;
        self.settle(at, run_id, landing)
    }

    /// Which room holds each node of a building's plan.
    ///
    /// Read from the plan and the claim records together: the table
    /// says which nodes are being worked on, and the records say from
    /// which room. A node marked `In progress` that no record claims is
    /// left out rather than guessed at.
    fn holders_in(&self, building: &Address) -> std::collections::BTreeMap<kernel::NodeId, String> {
        self.plan_holders.get(building).cloned().unwrap_or_default()
    }

    /// What could be started in one building right now.
    fn ready_in(&self, addr: &Address) -> Vec<kernel::NodeId> {
        self.plan_of(addr)
            .map(|tree| tree.ready())
            .unwrap_or_default()
    }

    /// One node's text, for the task a dispatch carries.
    fn plan_item(&self, addr: &Address, node: &kernel::NodeId) -> Option<String> {
        self.plan_of(addr)?
            .get(node)
            .map(|held| held.row.item.clone())
    }

    /// A building's plan as it stands on disk.
    ///
    /// Read here rather than folded: this runs once per dispatch, not
    /// once per question a page asks, and the worker writes the file it
    /// is reading.
    fn plan_of(&self, addr: &Address) -> Option<kernel::PlanTree> {
        let text = city::roadmap(&self.city_root, addr).ok()?;
        match kernel::check_roadmap_shape(&text) {
            kernel::RoadmapShape::WellFormed { rows } => kernel::PlanTree::build(rows).ok(),
            kernel::RoadmapShape::Malformed { .. } => None,
        }
    }

    /// Sets, pauses, resumes or clears a building's pursuit.
    ///
    /// **Held in the process and not written down.** A standing goal is
    /// a posture rather than a fact about the past, and a city that came
    /// back from a restart already working through the night is the one
    /// failure this must not have. What the goal *does* — every run it
    /// starts — is recorded like anything else.
    ///
    /// A goal is declared through the depth-zero position this Desk
    /// holds, which is the runtime half of the guard the type already
    /// carries: a sub-agent has no `Delegator` and cannot reach this
    /// function either.
    pub(super) fn set_pursuit(
        &mut self,
        addr: &Address,
        step: channels::PursuitStep,
    ) -> Result<(), AxError> {
        let missing = |action: &'static str| {
            AxError::failure(
                AxCode::InvalidArgs,
                action,
                format!("{} is not pursuing anything", addr.as_str()),
            )
            .with_recovery("set a goal first; there is nothing here to change")
        };
        let name = match step {
            channels::PursuitStep::Set { goal } => {
                // Declared through the depth-zero position this worker
                // holds. That is the runtime half of the guard the type
                // already carries: a sub-agent has no `Delegator`, and
                // no path from a tool reaches this function either.
                let declared = kernel::Pursuit::declare(&self.delegator, goal)?;
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "kernel::pursuit",
                    &format!(
                        "{} works towards `{}` until nothing is ready",
                        addr.as_str(),
                        declared.goal()
                    ),
                );
                self.pursuits.insert(addr.clone(), declared);
                "set"
            }
            channels::PursuitStep::Pause => {
                self.pursuits
                    .get_mut(addr)
                    .ok_or_else(|| missing("pause a pursuit"))?
                    .pause();
                "pause"
            }
            channels::PursuitStep::Resume => {
                self.pursuits
                    .get_mut(addr)
                    .ok_or_else(|| missing("resume a pursuit"))?
                    .resume();
                "resume"
            }
            channels::PursuitStep::Clear => {
                self.pursuits
                    .remove(addr)
                    .ok_or_else(|| missing("clear a pursuit"))?;
                "clear"
            }
        };
        let mut map = serde_json::Map::new();
        map.insert(
            "step".to_owned(),
            serde_json::Value::String(name.to_owned()),
        );
        if let Some(held) = self.pursuits.get(addr) {
            map.insert(
                "goal".to_owned(),
                serde_json::Value::String(held.goal().to_owned()),
            );
        }
        self.record_at(EventKind::PursuitChanged, addr.clone(), Payload::new(map)?)?;
        self.pursue(addr)
    }

    /// Takes ready work for as long as a pursuit says to.
    ///
    /// **It terminates because every dispatch takes a node out of the
    /// ready set.** Claiming moves a node to `In progress`, and a run
    /// that ends still holding one leaves it blocked, so the set this
    /// reads from strictly shrinks — except when a run splits a branch,
    /// which is the city finding more work rather than looping.
    ///
    /// The verdict is `kernel::pursuit`'s and is not re-derived here:
    /// what "there is nothing left to do" means has one authority.
    fn pursue(&mut self, addr: &Address) -> Result<(), AxError> {
        loop {
            let Some(state) = self.pursuits.get(addr).map(kernel::Pursuit::state) else {
                return Ok(());
            };
            let ready = self.ready_in(addr);
            let goal = self
                .pursuits
                .get(addr)
                .map(|held| held.goal().to_owned())
                .unwrap_or_default();
            match kernel::observe_pursuit(state, &ready, 0) {
                kernel::PursuitVerdict::Work { next } => {
                    let Some(node) = self.plan_item(addr, &next) else {
                        return Ok(());
                    };
                    self.note(
                        runtime::diagnostics::Level::Effect,
                        "kernel::pursuit",
                        &format!("{} takes {next}: {node}", addr.as_str()),
                    );
                    self.dispatch_in(
                        Assignment {
                            addr: addr.clone(),
                            session: None,
                            effort: None,
                            mode: runtime::Mode::PlanGoal,
                            parent: None,
                            succession: None,
                        },
                        format!("Plan node {next}: {node}"),
                        goal,
                    )?;
                    // A dispatch that did not move the node would loop
                    // for ever, so the check is on the ready set itself
                    // rather than on a counter.
                    if self.ready_in(addr).contains(&next) {
                        self.note(
                            runtime::diagnostics::Level::Refuse,
                            "kernel::pursuit",
                            &format!(
                                "{next} is still ready after a run took it; the pursuit stops \
                                 rather than dispatching it again"
                            ),
                        );
                        return Ok(());
                    }
                }
                kernel::PursuitVerdict::Waiting { .. }
                | kernel::PursuitVerdict::Paused
                | kernel::PursuitVerdict::Finished => return Ok(()),
            }
        }
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
