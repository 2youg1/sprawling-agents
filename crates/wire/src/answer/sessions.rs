// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stretches of one room, newest first: where each began, how, its
//! runs and its last line (wire-SPEC §8-71).

use kernel::{Address, Origin, Seq, TimeMs};
use serde::{Deserialize, Serialize};

use crate::command::Carry;

/// How many stretches one answer lists. A bound on the size of an answer
/// on the wire, not a machine reading: the page that reads it shows a
/// room's recent stretches, and `earlier` says how many more there are.
pub const SESSIONS_MAX: usize = 64;

/// One room's stretches, newest first, and how many older ones the answer
/// leaves out. `room` echoes the question, which the wire carries no id
/// for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionsAnswer {
    pub room: Address,
    pub sessions: Vec<SessionLine>,
    pub earlier: u64,
}

/// One stretch of a room, each field read off a line the Ledger files
/// under the room's address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionLine {
    /// The stretch's first line: its `session_opened`, or the room's first
    /// `run_started` when dispatching opened the room.
    pub began: Seq,
    pub start: SessionStart,
    /// The runs started in this stretch.
    pub runs: u64,
    /// The last line of this stretch filed under the room's address, and
    /// its time: the room's slice ends there (storage-SPEC 8-24).
    pub last: Seq,
    pub at: TimeMs,
}

/// How a stretch began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SessionStart {
    /// Dispatching into the room opened it; no `session_opened` was written.
    Dispatched,
    /// `/new`: what crossed from the stretch before - `Handoff` when the
    /// previous stretch's handoff travelled - and the run line it branched
    /// from, when it is a branch.
    Opened { carry: Carry, from: Option<Origin> },
}
