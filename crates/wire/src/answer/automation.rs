// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The schedule and the watch table as a page reads them
//! (wire-SPEC.md 8-62).

use kernel::Address;
use serde::{Deserialize, Serialize};

/// Both automation files, each in the order it was written, and the file
/// that did not read with its reason. A file that is not there states no
/// rows, which is a city that automates nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AutomationAnswer {
    pub jobs: Vec<ScheduledJob>,
    pub sources: Vec<WatchedSource>,
    /// One line per file that did not read: its name and the refusal the
    /// next tick would give. Its rows are absent rather than guessed.
    pub unreadable: Vec<String>,
}

/// One row of `SCHEDULE.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScheduledJob {
    pub name: String,
    pub addr: Address,
    pub task: String,
    pub goal: String,
    pub cadence: Cadence,
}

/// How often a scheduled job runs, in UTC minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Cadence {
    EveryMinutes {
        minutes: u64,
    },
    /// The minute of the day.
    DailyAt {
        minute: u64,
    },
    /// The minute of the week, Monday first.
    WeeklyAt {
        minute: u64,
    },
}

/// One row of `WATCH.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WatchedSource {
    pub name: String,
    /// The substring that routes an arrival here.
    pub matches: String,
    pub addr: Address,
    /// Whether an arrival from here may start work, or is only noticed.
    pub starts_work: bool,
}
