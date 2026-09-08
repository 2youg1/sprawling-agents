// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Roadmap rows: the types a table is read into.

use serde::{Deserialize, Serialize};

use crate::locator::Locator;
use crate::node_id::NodeId;

/// The five states, closed. The spellings below are the canonical table
/// strings, and there is exactly one set of them: a second set would be a
/// second authority over what a resident is allowed to write.
///
/// The serde spellings are `snake_case` identifiers rather than the table
/// words: a wire frame is read by a program and a table cell by a person,
/// and making the client carry `Awaiting approval` would put an English
/// sentence where the phrase table belongs.
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

/// Status column spellings — the parse table, data not code. These are
/// the words the template asks a resident to write.
pub const ROADMAP_STATUS_SPELLINGS: [(RoadmapStatus, &str); 5] = [
    (RoadmapStatus::NotStarted, "Not started"),
    (RoadmapStatus::InProgress, "In progress"),
    (RoadmapStatus::Done, "Done"),
    (RoadmapStatus::Blocked, "Blocked"),
    (RoadmapStatus::AwaitingApproval, "Awaiting approval"),
];

/// The number of columns the roadmap table has. Named because three
/// places state it — the parser, the refusal it writes, and the
/// template — and a plan whose columns disagree with its reader has no
/// denominator at all.
pub const ROADMAP_COLUMNS: usize = 6;

/// The name of the plan file, in every building at every depth.
///
/// Named here beside the table's own grammar because the file and the
/// table are one fact: `city` spells the path from it, and
/// `write_domain` refuses it to a documents domain, so a rename moves
/// one line rather than three.
pub const ROADMAP_FILE: &str = "Roadmap.md";

impl RoadmapStatus {
    /// The one spelling, for every writer and every message.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        ROADMAP_STATUS_SPELLINGS
            .iter()
            .find(|(known, _)| *known == self)
            .map_or("Not started", |(_, spelling)| *spelling)
    }
}

/// The evidence column, three-way: empty and invalid are different facts
/// — one renders as "in progress", the other as "suspect".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCell {
    Empty,
    Invalid { raw: String },
    Present(Locator),
}

/// One row, as the table spells it.
///
/// `weight` is a ratio among the rows that share a parent and never a
/// quantity, so doubling every number on one level changes nothing. An
/// empty cell reads as 1: a plan that says nothing about how a level
/// divides is a plan that divides it evenly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapRow {
    pub id: NodeId,
    pub item: String,
    pub weight: u32,
    pub needs: Vec<NodeId>,
    pub status: RoadmapStatus,
    pub evidence: EvidenceCell,
}

/// Deliberately exhaustive shape verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoadmapShape {
    WellFormed { rows: Vec<RoadmapRow> },
    Malformed { problems: Vec<String> },
}

/// A child to hang under a node: what it is, and how big a slice of its
/// parent it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChild {
    pub item: String,
    pub weight: u32,
}
