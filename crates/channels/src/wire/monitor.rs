// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance monitor's pair of frames (channels-SPEC.md 8-47):
//! whether a session is watching, and one reading sent to it.

use serde::{Deserialize, Serialize};

/// Whether this session counts as somebody watching the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Monitoring {
    /// Watch the whole monitor page.
    Watch,
    /// Watch only the fact bar's summary, which reads this process alone.
    WatchSummary,
    Release,
}

/// What one watcher looks at, which decides how much the city reads for
/// it. It does not travel: it is what `MonitorFeed::watch` is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Watched {
    Everything,
    Summary,
}

/// One reading of every counter the monitor shows, in integers because
/// it travels on the wire (sprawling-SPEC.md 8-94).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Sample {
    pub core_cpu_permille: u64,
    pub core_private_bytes: u64,
    pub core_working_set_bytes: u64,
    pub core_read_bytes: u64,
    pub core_written_bytes: u64,
    pub machine_cpu_permille: u64,
    pub machine_available_bytes: u64,
    pub volume_free_bytes: u64,
    pub ledger_queue_depth: u64,
    pub durable_lag: u64,
    pub relay_p50_nanos: u64,
    pub event_to_screen_p50_nanos: u64,
    pub queued_runs: u64,
}
