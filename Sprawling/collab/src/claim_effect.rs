// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{RoadmapMoved, RoadmapStep};
use kernel::spine::{append_top_level, check_roadmap_shape, insert_children, set_roadmap_status};
use kernel::{AxError, NewChild, NodeId, Payload, PlanExit};
use kernel::{RoadmapShape, RoadmapStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimEffect {
    Claimed {
        id: NodeId,
        item: String,
    },

    PutDown {
        id: NodeId,
        item: String,
        exit: PlanExit,
    },

    Split {
        parent: NodeId,
        children: Vec<NewChild>,
    },

    Added {
        id: NodeId,
        child: NewChild,
    },
}

impl ClaimEffect {
    #[must_use]
    pub fn id(&self) -> &NodeId {
        match self {
            ClaimEffect::Claimed { id, .. }
            | ClaimEffect::PutDown { id, .. }
            | ClaimEffect::Added { id, .. } => id,
            ClaimEffect::Split { parent, .. } => parent,
        }
    }

    pub fn apply(&self, text: &str) -> Result<String, AxError> {
        match self {
            ClaimEffect::Claimed { id, .. } => {
                set_roadmap_status(text, id, RoadmapStatus::InProgress, None)
            }
            ClaimEffect::PutDown { id, exit, .. } => {
                set_roadmap_status(text, id, exit.status(), exit.evidence())
            }
            ClaimEffect::Split { parent, children } => insert_children(text, parent, children),
            ClaimEffect::Added { id, child } => append_top_level(text, id, child),
        }
    }

    #[must_use]
    pub fn kind(&self) -> Option<kernel::EventKind> {
        Some(match self {
            ClaimEffect::Claimed { .. } => kernel::EventKind::RoadmapClaimed,
            ClaimEffect::Split { .. } => kernel::EventKind::RoadmapSplit,
            ClaimEffect::PutDown { exit, .. } => match exit {
                PlanExit::Finished { .. } => kernel::EventKind::RoadmapFinished,
                PlanExit::Stopped { why, .. } if why.is_red() => kernel::EventKind::RoadmapBlocked,
                PlanExit::Stopped { .. } => kernel::EventKind::RoadmapReleased,
            },
            ClaimEffect::Added { .. } => return None,
        })
    }

    pub fn line(&self, who: &str) -> Result<Option<(kernel::EventKind, Payload)>, AxError> {
        match self.kind() {
            Some(kind) => Ok(Some((kind, self.payload(who)?))),
            None => Ok(None),
        }
    }

    fn payload(&self, who: &str) -> Result<Payload, AxError> {
        let step = match self {
            ClaimEffect::Added { id, .. } => {
                return Err(AxError::failure(
                    kernel::AxCode::InvalidArgs,
                    "record a plan step",
                    format!("the row added as {id} is written with the plan file, not as a line"),
                )
                .with_recovery("write the plan file; an added row has no record of its own"));
            }
            ClaimEffect::Claimed { item, .. } => RoadmapStep::Claimed { item: item.clone() },
            ClaimEffect::Split { children, .. } => RoadmapStep::Split {
                children: children.iter().map(|child| child.item.clone()).collect(),
            },
            ClaimEffect::PutDown { item, exit, .. } => {
                let item = item.clone();
                match exit {
                    PlanExit::Finished { evidence, .. } => RoadmapStep::Finished {
                        item,
                        evidence: evidence.clone(),
                    },
                    PlanExit::Stopped { why, .. } if why.is_red() => RoadmapStep::Blocked {
                        item,
                        why: why.clone(),
                        line: why.line(),
                    },
                    PlanExit::Stopped { why, .. } => RoadmapStep::Released {
                        item,
                        why: why.clone(),
                        line: why.line(),
                    },
                }
            }
        };
        Payload::of(&RoadmapMoved {
            by: who.to_owned(),
            node: self.id().clone(),
            step,
        })
    }
}

#[must_use]
pub fn still_true(text: &str, effect: &ClaimEffect) -> bool {
    match effect {
        ClaimEffect::Claimed { id, .. } => {
            let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
                return false;
            };
            rows.iter()
                .find(|row| &row.id == id)
                .is_some_and(|row| row.status == RoadmapStatus::NotStarted)
        }
        ClaimEffect::PutDown { .. } | ClaimEffect::Split { .. } | ClaimEffect::Added { .. } => true,
    }
}
