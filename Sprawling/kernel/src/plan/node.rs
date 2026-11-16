// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::locator::Locator;
use crate::node_id::NodeId;
use crate::share::Share;
use crate::spine::{RoadmapRow, RoadmapStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopCause {
    Blocked { note: String },

    HandedBack { note: String },

    FrozeWithoutEvidence,

    Stalled { repeats: u32 },

    GateOverdue { waited_ms: u64 },
}

impl StopCause {
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

    #[must_use]
    pub fn is_red(&self) -> bool {
        !matches!(self, StopCause::HandedBack { .. })
    }

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
                format!("a door has waited {waited_ms} ms for the User")
            }
        }
    }
}

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

    pub fn finish(self, evidence: Locator) -> PlanExit {
        PlanExit::Finished {
            id: self.0,
            evidence,
        }
    }

    pub fn stop(self, why: StopCause) -> PlanExit {
        PlanExit::Stopped { id: self.0, why }
    }
}

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

    #[must_use]
    pub fn status(&self) -> RoadmapStatus {
        match self {
            PlanExit::Finished { .. } => RoadmapStatus::Done,
            PlanExit::Stopped { why, .. } => why.status(),
        }
    }

    #[must_use]
    pub fn evidence(&self) -> Option<&Locator> {
        match self {
            PlanExit::Finished { evidence, .. } => Some(evidence),
            PlanExit::Stopped { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub row: RoadmapRow,

    pub share: Share,

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
