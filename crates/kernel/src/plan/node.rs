// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Plan nodes: the types a plan is made of, without the tree that hangs them.

use serde::{Deserialize, Serialize};

use crate::locator::Locator;
use crate::node_id::NodeId;
use crate::share::Share;
use crate::spine::{RoadmapRow, RoadmapStatus};

/// Why a held node was put down without evidence.
///
/// Exhaustive, and the whole point of the enum is the division it draws:
/// a resident that hands work back leaves it ready for somebody else,
/// while every other cause leaves it red.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopCause {
    /// A resident says this node cannot proceed, and why.
    Blocked { note: String },
    /// A resident is not the one to do this after all. Not red: the node
    /// goes back where another run can take it.
    HandedBack { note: String },
    /// The run holding it ended without citing anything.
    FrozeWithoutEvidence,
    /// `crate::stall` found the run going nowhere.
    Stalled { repeats: u32 },
    /// A door was escalated to a person and nobody has answered.
    GateOverdue { waited_ms: u64 },
}

impl StopCause {
    /// What the status column says afterwards.
    #[must_use]
    pub fn status(&self) -> RoadmapStatus {
        match self {
            StopCause::HandedBack { .. } => RoadmapStatus::NotStarted,
            StopCause::Blocked { .. }
            | StopCause::FrozeWithoutEvidence
            | StopCause::Stalled { .. }
            | StopCause::GateOverdue { .. } => RoadmapStatus::Blocked,
        }
    }

    /// Whether this stop is one the plan reports as red.
    #[must_use]
    pub fn is_red(&self) -> bool {
        !matches!(self, StopCause::HandedBack { .. })
    }

    /// One line, for a person reading the first screen.
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            StopCause::Blocked { note } | StopCause::HandedBack { note } => note.clone(),
            StopCause::FrozeWithoutEvidence => {
                "the run holding it ended without citing anything".to_owned()
            }
            StopCause::Stalled { repeats } => {
                format!("the run repeated one action {repeats} times")
            }
            StopCause::GateOverdue { waited_ms } => {
                format!("a door has waited {waited_ms} ms for a person")
            }
        }
    }
}

/// A node this run holds.
///
/// **There is no third way to put it down.** The type has one private
/// field, so it is minted only by [`PlanTree::claim`], and it is
/// consumed only by [`Held::finish`] and [`Held::stop`] — one takes
/// evidence, the other takes a cause. A run that simply stops working
/// leaves the value with its owner, and the owner's freeze path spends
/// it on [`StopCause::FrozeWithoutEvidence`], which is where the red in
/// `crate::blockage` comes from.
#[derive(Debug)]
#[must_use = "a held node has to be finished with evidence or stopped with a cause"]
pub struct Held(NodeId);

impl Held {
    pub(crate) fn of(id: NodeId) -> Self {
        Held(id)
    }

    #[must_use]
    pub fn id(&self) -> &NodeId {
        &self.0
    }

    /// Green: the work is done and here is where to look.
    pub fn finish(self, evidence: Locator) -> PlanExit {
        PlanExit::Finished {
            id: self.0,
            evidence,
        }
    }

    /// Red, or back in the pool: either way it says why.
    pub fn stop(self, why: StopCause) -> PlanExit {
        PlanExit::Stopped { id: self.0, why }
    }
}

/// How a held node left. Exhaustive, and every arm carries its reason —
/// which is the whole of the plan gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanExit {
    Finished { id: NodeId, evidence: Locator },
    Stopped { id: NodeId, why: StopCause },
}

impl PlanExit {
    #[must_use]
    pub fn id(&self) -> &NodeId {
        match self {
            PlanExit::Finished { id, .. } | PlanExit::Stopped { id, .. } => id,
        }
    }

    /// The status the row carries after this exit.
    #[must_use]
    pub fn status(&self) -> RoadmapStatus {
        match self {
            PlanExit::Finished { .. } => RoadmapStatus::Done,
            PlanExit::Stopped { why, .. } => why.status(),
        }
    }

    /// The evidence cell after this exit: an exit without evidence
    /// clears it rather than leaving the last run's citation on a row
    /// that is no longer done.
    #[must_use]
    pub fn evidence(&self) -> Option<&Locator> {
        match self {
            PlanExit::Finished { evidence, .. } => Some(evidence),
            PlanExit::Stopped { .. } => None,
        }
    }
}

/// One node, placed: the row as written plus what the tree worked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub row: RoadmapRow,
    /// This node's part of the whole plan.
    pub share: Share,
    /// Its children, in table order. Empty means a leaf, and only
    /// leaves carry progress.
    pub children: Vec<NodeId>,
}

impl PlanNode {
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::PlanTree;
    use super::*;
    use crate::spine::{RoadmapShape, check_roadmap_shape};
    fn tree(text: &str) -> PlanTree {
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
            panic!("the fixture parses");
        };
        PlanTree::build(rows).expect("the fixture is a tree")
    }
    const HEAD: &str = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
";
    fn locator() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap()
    }

    /// The plan gate: a held node leaves green with evidence or stopped
    /// with a cause, and handing back is the one stop that is not red.
    #[test]
    fn a_held_node_leaves_by_one_of_two_doors() {
        let plan = tree(&format!(
            "{HEAD}\
| 1 | frame | 1 |  | Not started | |
| 2 | roof | 1 |  | Not started | |
| 3 | door | 1 |  | Not started | |
"
        ));
        let green = plan
            .claim(&NodeId::parse("1").unwrap())
            .unwrap()
            .finish(locator());
        assert_eq!(green.status(), RoadmapStatus::Done);
        assert!(green.evidence().is_some());

        let red = plan
            .claim(&NodeId::parse("2").unwrap())
            .unwrap()
            .stop(StopCause::Blocked {
                note: "the kiln is cold".to_owned(),
            });
        assert_eq!(red.status(), RoadmapStatus::Blocked);
        assert!(red.evidence().is_none(), "a stop clears the evidence cell");

        let back = plan
            .claim(&NodeId::parse("3").unwrap())
            .unwrap()
            .stop(StopCause::HandedBack {
                note: "not mine".to_owned(),
            });
        assert_eq!(
            back.status(),
            RoadmapStatus::NotStarted,
            "handing back returns the node to the ready set"
        );
        let PlanExit::Stopped { why, .. } = &back else {
            panic!("stopped")
        };
        assert!(!why.is_red(), "and it is not red");
    }

    #[test]
    fn every_stop_cause_says_something_a_person_can_read() {
        for cause in [
            StopCause::Blocked {
                note: "no key".to_owned(),
            },
            StopCause::HandedBack {
                note: "not mine".to_owned(),
            },
            StopCause::FrozeWithoutEvidence,
            StopCause::Stalled { repeats: 3 },
            StopCause::GateOverdue { waited_ms: 60_000 },
        ] {
            assert!(!cause.line().is_empty(), "{cause:?}");
        }
    }
}
