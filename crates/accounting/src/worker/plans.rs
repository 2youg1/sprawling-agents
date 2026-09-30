// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One building's plan: who holds what, and how far red reaches.

use kernel::event::record::{PursuitChanged, PursuitMove};
use kernel::{Address, AxCode, AxError, EventKind};
use kernel::{Payload, RunId};

use crate::effect;

use super::{Assignment, RunWorker};

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
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "one claim effect stops a node red; the others do not stop one"
    )]
    pub(super) fn tell_whoever_is_behind(
        &mut self,
        at: &Assignment,
        by: Reporter<'_>,
        effects: &[collab::ClaimEffect],
        chain: &super::KnockChain,
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
        for notice in kernel::blockage::notices(&kernel::blockage::spread(&tree, &red), &held) {
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
                kernel::event::record::SignalId::parse(&format!(
                    "blocked-{}-{}",
                    building.as_str().replace('/', "-"),
                    notice.about
                ))?,
                kernel::event::record::SignalKind::Mention,
                who.to_owned(),
                to,
                kernel::Version::FIRST,
                Payload::new(payload)?,
                self.clock.now()?,
            )?));
        }
        if sent.is_empty() {
            return Ok(());
        }
        let landing = effect::Landing::signals(sent, room, who)?;
        self.settle(at, run_id, landing, chain)
    }

    /// Which room holds each node of a building's plan.
    ///
    /// Read from the plan and the claim records together: the table
    /// says which nodes are being worked on, and the records say from
    /// which room. A node marked `In progress` that no record claims is
    /// left out rather than guessed at.
    fn holders_in(&self, building: &Address) -> std::collections::BTreeMap<kernel::NodeId, String> {
        self.planning.holders.in_building(building)
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
        match kernel::spine::check_roadmap_shape(&text) {
            kernel::RoadmapShape::WellFormed { rows } => kernel::PlanTree::build(rows).ok(),
            kernel::RoadmapShape::Malformed { .. } => None,
        }
    }

    /// Refuses a standing goal on a building that has no plan to work
    /// through, saying why.
    ///
    /// Only an absent or empty plan is `E_PLAN_MISSING`, because that
    /// code's recovery asks the mayor to write one. A read failure keeps
    /// its own code, and a plan that does not parse is `E_INVALID_ARGS`
    /// naming its broken lines, so a plan the person wrote is never
    /// offered to be written over.
    fn require_plan_to_pursue(&self, addr: &Address, goal: &str) -> Result<(), AxError> {
        const ACTION: &str = "set a standing goal";
        let missing = || {
            AxError::failure(
                AxCode::PlanMissing,
                ACTION,
                format!("{}: {goal}", addr.as_str()),
            )
            .with_recovery("ask the mayor to write this building's plan, then set the goal again")
        };
        let text = city::roadmap(&self.city_root, addr)?;
        if text.trim().is_empty() {
            return Err(missing());
        }
        let plan = match kernel::spine::check_roadmap_shape(&text) {
            kernel::RoadmapShape::WellFormed { rows } => kernel::PlanTree::build(rows)?,
            kernel::RoadmapShape::Malformed { problems } => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    ACTION,
                    format!("{}: {}", addr.as_str(), problems.join("; ")),
                )
                .with_recovery(
                    "repair the table in this building's plan, then set the goal again",
                ));
            }
        };
        if plan.is_empty() {
            return Err(missing());
        }
        Ok(())
    }

    /// Sets, pauses, resumes or clears a building's pursuit.
    ///
    /// **Held in the process only once the ledger has it.** Each change
    /// is decided and its pursuit minted first, then `pursuit_changed` is
    /// appended, then the worker holds the result, so a failed append
    /// leaves nothing a restart would not fold back (sprawling-SPEC.md
    /// 8-111).
    ///
    /// A goal is declared through the depth-zero position this Desk
    /// holds, which is the runtime half of the guard the type already
    /// carries: a sub-agent has no `Delegator` and cannot reach this
    /// function either.
    pub(super) fn set_pursuit(
        &mut self,
        addr: &Address,
        step: wire::PursuitStep,
    ) -> Result<(), AxError> {
        let missing = |action: &'static str| {
            AxError::failure(
                AxCode::InvalidArgs,
                action,
                format!("{} is not pursuing anything", addr.as_str()),
            )
            .with_recovery("set a goal first; there is nothing here to change")
        };
        // Decided and minted first, held only once the ledger has the
        // line: a failed append leaves the process holding what a restart
        // would fold (sprawling-SPEC.md 8-111).
        let change = match step {
            wire::PursuitStep::Set { goal } => {
                // A pursuit works through the plan's ready steps, so on a
                // building with no plan it would finish at once having
                // done nothing. The subject is `<building>: <goal>`, the
                // shape the client's form recovery reads to prefill the
                // mayor's request for a plan (client-SPEC 4-35a).
                self.require_plan_to_pursue(addr, &goal)?;
                // Declared through the depth-zero position this worker
                // holds. That is the runtime half of the guard the type
                // already carries: a sub-agent has no `Delegator`, and
                // no path from a tool reaches this function either.
                PursuitChange::Declare(kernel::Pursuit::declare(&self.planning.delegator, goal)?)
            }
            wire::PursuitStep::Pause => PursuitChange::Pause,
            wire::PursuitStep::Resume => PursuitChange::Resume,
            wire::PursuitStep::Clear => PursuitChange::Clear,
        };
        let held = self.planning.pursuits.get(addr);
        let changed = match &change {
            PursuitChange::Declare(pursuit) => PursuitChanged {
                step: PursuitMove::Set,
                goal: Some(pursuit.goal().to_owned()),
            },
            PursuitChange::Pause => PursuitChanged {
                step: PursuitMove::Pause,
                goal: Some(
                    held.ok_or_else(|| missing("pause a pursuit"))?
                        .goal()
                        .to_owned(),
                ),
            },
            PursuitChange::Resume => PursuitChanged {
                step: PursuitMove::Resume,
                goal: Some(
                    held.ok_or_else(|| missing("resume a pursuit"))?
                        .goal()
                        .to_owned(),
                ),
            },
            PursuitChange::Clear => {
                held.ok_or_else(|| missing("clear a pursuit"))?;
                PursuitChanged {
                    step: PursuitMove::Clear,
                    goal: None,
                }
            }
        };
        self.record_at(
            EventKind::PursuitChanged,
            addr.clone(),
            Payload::of(&changed)?,
        )?;
        self.hold_pursuit(addr, change);
        self.pursue(addr)
    }

    /// Applies one pursuit change the ledger already holds to the
    /// pursuit this worker keeps for `addr`.
    fn hold_pursuit(&mut self, addr: &Address, change: PursuitChange) {
        match change {
            PursuitChange::Declare(pursuit) => {
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "kernel::pursuit",
                    &format!(
                        "{} works towards `{}` until nothing is ready",
                        addr.as_str(),
                        pursuit.goal()
                    ),
                );
                self.planning.pursuits.insert(addr.clone(), pursuit);
            }
            PursuitChange::Pause => {
                if let Some(held) = self.planning.pursuits.get_mut(addr) {
                    held.pause();
                }
            }
            PursuitChange::Resume => {
                if let Some(held) = self.planning.pursuits.get_mut(addr) {
                    held.resume();
                }
            }
            PursuitChange::Clear => {
                self.planning.pursuits.remove(addr);
            }
        }
    }
}

/// One step of a building's pursuit, decided and minted before its
/// `pursuit_changed` line is written and held only after.
enum PursuitChange {
    Declare(kernel::Pursuit),
    Pause,
    Resume,
    Clear,
}

pub(super) mod held;
mod pursuing;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
