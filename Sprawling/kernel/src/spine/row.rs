// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::locator::Locator;
use crate::node_id::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RoadmapStatus {
    NotStarted,
    InProgress,
    Done,
    Blocked,
    AwaitingApproval,
}

pub const ROADMAP_STATUS_SPELLINGS: [(RoadmapStatus, &str); 5] = [
    (RoadmapStatus::NotStarted, "Not started"),
    (RoadmapStatus::InProgress, "In progress"),
    (RoadmapStatus::Done, "Done"),
    (RoadmapStatus::Blocked, "Blocked"),
    (RoadmapStatus::AwaitingApproval, "Awaiting approval"),
];

pub const ROADMAP_COLUMNS: usize = 6;

pub const ROADMAP_FILE: &str = "Roadmap.md";

impl RoadmapStatus {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        ROADMAP_STATUS_SPELLINGS
            .iter()
            .find(|(known, _)| *known == self)
            .map_or("Not started", |(_, spelling)| *spelling)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCell {
    Empty,
    Invalid { raw: String },
    Present(Locator),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapRow {
    pub id: NodeId,
    pub item: String,
    pub weight: u32,
    pub needs: Vec<NodeId>,
    pub status: RoadmapStatus,
    pub evidence: EvidenceCell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoadmapShape {
    WellFormed { rows: Vec<RoadmapRow> },
    Malformed { problems: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChild {
    pub item: String,
    pub weight: u32,
}
