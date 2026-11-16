// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One run as a reader who never saw its stream needs it
//! (`crates/wire/spec/Reading.lean` D34 for `waiting`).

use kernel::{Address, EventKind, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

/// One run, as a reader needs it. The client folds the live stream for
/// itself; this shape is what a query answers about runs it never saw,
/// which is why it carries the position rather than the whole history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunSummary {
    pub run: RunId,
    pub who: String,
    pub frozen: bool,
    pub last_seq: Seq,
    pub last_kind: EventKind,
    /// The room the run works in, from its `run_started` record. `who`
    /// cannot say it: that is the author of the first record, which is
    /// the city. Absent when the view saw no opening for this run.
    pub addr: Option<Address>,
    /// When the run began, from the same record.
    pub started: Option<TimeMs>,
    /// How the run ended, as its `run_frozen` record says. Absent while
    /// it runs, and when the view never saw the freeze.
    pub completion: Option<String>,
    /// The branch of the last pull request the run opened; a pull
    /// request in the city is named by its branch and has no number.
    pub pr: Option<String>,
    /// What the run waits for the person to allow. Present exactly when
    /// `last_kind` is `approval_requested`.
    pub ask: Option<String>,
    /// What the person asked for, as the run's `run_started` record says.
    /// Absent when the view never saw the opening or the task was empty.
    pub task: Option<String>,
    /// What finishing looks like, from the same record, read the same way.
    pub goal: Option<String>,
    /// The room the run stopped on a synchronous `send` to hear from,
    /// and until when; absent while it waits for nobody, and always
    /// absent once it froze (`crates/wire/spec/Reading.lean` D34).
    /// Defaulted, so an answer from a build without the field still
    /// reads.
    #[serde(default)]
    pub waiting: Option<Waiting>,
}

/// What a run waits for: the room it spoke to, and the instant on the
/// city's clock at which the wait ends without a reply. An instant
/// rather than the time left, because an answer is cached and carried,
/// and a count of milliseconds left is wrong once it leaves the city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Waiting {
    pub on: Address,
    pub until: TimeMs,
}
