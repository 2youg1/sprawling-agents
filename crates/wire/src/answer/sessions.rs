// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The stretches of one room, newest first: where each began, how, its
//! runs and its last line (`crates/wire/spec/Answer/Sessions.lean` §8-71).

use kernel::{Address, Effort, Origin, Seq, TimeMs};
use serde::{Deserialize, Serialize};

use crate::command::Carry;

/// How many stretches one answer lists. A bound on the size of an answer
/// on the wire, not a machine reading: the page that reads it shows a
/// room's recent stretches, and `earlier` says how many more there are.
pub const SESSIONS_MAX: usize = 64;

/// How many characters of a stretch's last reply [`SessionLine::preview`]
/// carries. A bound on the answer, not on what a page shows: the page cuts
/// again to the width it has (`crates/wire/spec/Answer/Sessions.lean` D27).
pub const SESSION_PREVIEW_MAX: usize = 240;

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
    /// its time: the room's slice ends there (`crates/storage/Spec.lean` §8-24).
    pub last: Seq,
    pub at: TimeMs,
    /// The name the last `session_named` for this stretch gave it; `None`
    /// when it has none, and a page shows the room's address.
    #[serde(default)]
    pub name: Option<String>,
    /// The model the stretch's last run called.
    #[serde(default)]
    pub model: Option<String>,
    /// The effort the stretch's last run froze; `None` when it left the
    /// provider to choose, and on lines written before the key existed.
    #[serde(default)]
    pub effort: Option<Effort>,
    /// The worktree the stretch's last run was lent.
    #[serde(default)]
    pub workspace: Option<String>,
    /// The start of the stretch's last reply, at most
    /// [`SESSION_PREVIEW_MAX`] characters.
    #[serde(default)]
    pub preview: Option<String>,
}

/// How a stretch began.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SessionStart {
    /// Dispatching into the room opened it; no `session_opened` was
    /// written. `by` is who dispatched, as that first `run_started`
    /// records it; `None` for a line written before the key existed (D40).
    Dispatched {
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        by: Option<kernel::event::Who>,
    },
    /// `/new`: what crossed from the stretch before - `Handoff` when the
    /// previous stretch's handoff travelled - and the run line it branched
    /// from, when it is a branch.
    Opened { carry: Carry, from: Option<Origin> },
}
