// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building says about itself: the rows of its plan, what is
//! stuck and how much waits behind it, the goal it works towards, the
//! documents it keeps and what it has filed. Read shapes only, cut out of
//! `answer` so that file stays inside its budget; every name is
//! re-exported from there and the public spelling is unchanged.

use kernel::{Address, McpServer, NodeId, Progress, PursuitState, RoadmapStatus, SandboxLimits};
use serde::{Deserialize, Serialize};

/// A building's plan, as its own `Roadmap.md` states it.
///
/// `problems` carries the rows the table could not state — a row with a
/// status outside the five words, a column count that is not six, or a
/// dependency that runs in a circle. The interface shows them: a plan
/// that quietly drops the lines it could not parse would report progress
/// against a denominator nobody chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingProgress {
    pub addr: Address,
    pub progress: Progress,
    pub problems: Vec<String>,
    /// What is stuck, one line per cause. Never one per symptom: a plan
    /// with one real problem sends one line, and the seventeen nodes
    /// waiting behind it are a count on that line rather than seventeen
    /// more of them.
    pub blocked: Vec<BlockedLine>,
    /// How many nodes could be started right now. The number that says
    /// whether a city with a standing goal has anything left to do.
    pub ready: u32,
}

/// One cause, and how much is waiting behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BlockedLine {
    /// The node it is stuck at.
    pub source: NodeId,
    /// The whole sentence: which branch, which node, and why.
    pub line: String,
    /// How many other nodes cannot move until this one does.
    pub waiting: u32,
}

/// One node of a building's plan, flattened for a renderer.
///
/// The tree travels as rows in reading order rather than as nested
/// objects: every face the client draws — the list, the board, a branch
/// summary — wants a different grouping, and a shape that favoured one
/// of them would make the others re-flatten it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PlanRow {
    pub node: NodeId,
    pub item: String,
    pub status: RoadmapStatus,
    /// This node's share of the whole plan, in billionths.
    pub share_ppb: u64,
    pub needs: Vec<NodeId>,
    /// Whether a run could take it right now.
    pub ready: bool,
    /// Whether it carries work of its own. Only leaves are counted, so a
    /// renderer that showed branches in the same column as leaves would
    /// be showing the same work twice.
    pub leaf: bool,
    pub evidence: Option<String>,
}

/// A city's standing goal, if it has one.
///
/// `verdict` is the city's own reading of whether there is anything left
/// to do, so a page never has to work out the stop condition for itself
/// — which is what would give the condition a second authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PursuitLine {
    pub addr: Address,
    pub goal: String,
    pub state: PursuitState,
    /// One clause: working on 2.3, waiting for two runs, paused, or
    /// finished.
    pub verdict: String,
}

/// One document of a building, as it stands on disk.
///
/// The text is bounded: these files are written by agents over months,
/// and a page that ships an unbounded file has no answer for the day one
/// of them reaches a hundred megabytes. When it is cut, it says so -
/// silence about a cut is the difference between a view and a lie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingDoc {
    pub name: String,
    pub text: String,
    pub bytes: u64,
    pub truncated: bool,
}

/// One line of a building's archive index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArchiveLine {
    pub kind: String,
    /// Whole days since the epoch, as the archive files them.
    pub day: u64,
    pub subject: String,
}

/// What one building is: its plan, its own documents, its rooms and what
/// it has filed. The answer a person reads when they ask "what happened
/// in there".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingAnswer {
    pub addr: Address,
    pub progress: Progress,
    /// Rows of the plan this build could not read. Shown, never dropped.
    pub problems: Vec<String>,
    /// The plan tree, in reading order. Empty when the plan does not
    /// parse, which `problems` then explains.
    pub plan: Vec<PlanRow>,
    /// What is stuck, one line per cause.
    pub blocked: Vec<BlockedLine>,
    pub rooms: Vec<String>,
    pub docs: Vec<BuildingDoc>,
    pub archive: Vec<ArchiveLine>,
    /// What this building's own layer states its runs may reach. The
    /// resolved value is the ladder's; this is the rung a person edits,
    /// so a form that showed the resolved value would silently rewrite
    /// what a city-wide setting had said.
    pub sandbox: Option<SandboxLimits>,
    pub mcp: Vec<McpServer>,
}
