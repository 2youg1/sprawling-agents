// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a claim on a plan node left behind, and whether the file still
//! agrees with it.
//!
//! **A value, not a decision.** `ClaimEffect` records what happened so
//! the desk that produced it does not have to be consulted again, and
//! `still_true` asks the one question that cannot be answered from the
//! record alone: does the plan as it stands still leave the row a claim
//! took free to take. `ClaimDesk` decides; this describes and checks.
//! The properties both halves hold are proved in
//! `crates/collab/spec/Claim.lean`.
//! Two shapes, so two files (ARCHITECTURE.md section 9).

use kernel::event::record::{RoadmapMoved, RoadmapStep};
use kernel::spine::{append_top_level, check_roadmap_shape, insert_children, set_roadmap_status};
use kernel::{AxError, NewChild, NodeId, Payload, PlanExit};
use kernel::{RoadmapShape, RoadmapStatus};

/// What the run did to the plan. Exhaustive on purpose, like the other
/// desks': every variant is a line the worker has to write, so a new one
/// must be a compile error where the writing happens rather than a
/// runtime arm nobody reaches until a claim quietly goes unrecorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimEffect {
    Claimed {
        id: NodeId,
        item: String,
    },
    /// A held node put down. Carries the exit rather than a verb,
    /// because the exit is the thing the plan gate produced and copying
    /// its two arms into a second enum would be a second opinion about
    /// how a node may leave.
    PutDown {
        id: NodeId,
        item: String,
        exit: PlanExit,
    },
    /// Carries each child's weight as well as its item, because the
    /// worker replays the split onto the plan it reads at landing and a
    /// name alone does not rebuild the row the desk wrote.
    Split {
        parent: NodeId,
        children: Vec<NewChild>,
    },
    /// A top-level row written under the plan's root by the root's
    /// holder (kernel `Share.lean` D25). It carries the index the desk
    /// chose, because a claim this run makes next names the row by it.
    /// It is no ledger line of its own: the row rides on the plan file's
    /// write, which is how the plan reached the ledger before any row
    /// existed to claim.
    Added {
        id: NodeId,
        child: NewChild,
    },
}

impl ClaimEffect {
    /// The node the effect concerns, which is what the worker re-reads
    /// the file against before writing.
    #[must_use]
    pub fn id(&self) -> &NodeId {
        match self {
            ClaimEffect::Claimed { id, .. }
            | ClaimEffect::PutDown { id, .. }
            | ClaimEffect::Added { id, .. } => id,
            ClaimEffect::Split { parent, .. } => parent,
        }
    }

    /// The plan text with this effect written into it. The one
    /// definition of how an effect edits the plan: the desk edits its
    /// copy through it when the model calls, and the worker replays the
    /// same effects onto the file as it stands when the run lands, so a
    /// row another run landed meanwhile is kept rather than reverted.
    ///
    /// # Errors
    /// Propagates the plan's refusal to take the edit: a node that is
    /// not in the text, or a split that would not fit.
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

    /// Which record this becomes, or `None` for an added row, which
    /// rides on the plan file's write and is no line of its own.
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

    /// The record this becomes, kind and payload together, or `None`
    /// for an added row (see [`Self::kind`]).
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn line(&self, who: &str) -> Result<Option<(kernel::EventKind, Payload)>, AxError> {
        match self.kind() {
            Some(kind) => Ok(Some((kind, self.payload(who)?))),
            None => Ok(None),
        }
    }

    /// The `roadmap_*` payload: what a rebuild reads back.
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

/// Whether `text` still admits the effect. The worker asks this of each
/// effect against the plan the earlier ones left, so a concurrent run
/// degrades to "the second claim did not take and said so" rather than
/// "two runs each believe they own the node".
///
/// Only a claim can be untrue: it takes a row from the shared plan, so
/// the row must still be `Not started`. A put-down or a split acts only on
/// the row this run holds (collab D6), and the claim that took that row
/// was asked first; judging them again here would be a second answer to
/// whether they needed the row held.
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
