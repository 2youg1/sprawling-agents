// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{PursuitChanged, PursuitMove};
use kernel::{Address, AxCode, AxError, EventKind};
use kernel::{Payload, RunId};

use crate::effect;

use super::{Assignment, RunWorker};

#[derive(Clone, Copy)]
pub(super) struct Reporter<'a> {
    pub(super) run_id: RunId,
    pub(super) building: &'a Address,
    pub(super) who: &'a str,
}

impl RunWorker {
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

    fn holders_in(&self, building: &Address) -> std::collections::BTreeMap<kernel::NodeId, String> {
        self.planning.holders.in_building(building)
    }

    fn ready_in(&self, addr: &Address) -> Vec<kernel::NodeId> {
        self.plan_of(addr)
            .map(|tree| tree.ready())
            .unwrap_or_default()
    }

    fn plan_item(&self, addr: &Address, node: &kernel::NodeId) -> Option<String> {
        self.plan_of(addr)?
            .get(node)
            .map(|held| held.row.item.clone())
    }

    fn plan_of(&self, addr: &Address) -> Option<kernel::PlanTree> {
        let text = city::roadmap(&self.city_root, addr).ok()?;
        match kernel::spine::check_roadmap_shape(&text) {
            kernel::RoadmapShape::WellFormed { rows } => kernel::PlanTree::build(rows).ok(),
            kernel::RoadmapShape::Malformed { .. } => None,
        }
    }

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

        let change = match step {
            wire::PursuitStep::Set { goal } => {
                self.require_plan_to_pursue(addr, &goal)?;

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
