// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Three-layer config resolution and the frozen/live split. `FrozenConfig` and `LiveConfig` share no field: the freeze
//! line is a machine-checkable property, not a review note.

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::consts_policy::CLOCK_STAMP_DEFAULT;
use crate::model::Effort;
use crate::tool::ServerLabel;

/// Clock-stamp cadence for result envelopes. The
/// granularity rides the enum; `Off` costs zero bytes in the window.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockStampGranularity {
    Off,
    Minute,
    FiveMinute,
    Hour,
}

/// One value across the City -> Building -> Resident override ladder.
/// Lower layers win; absence falls through upward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredValue<T> {
    pub city: Option<T>,
    pub building: Option<T>,
    pub resident: Option<T>,
}

// Manual impl: the derive would demand `T: Default`, an undesired bound —
// an all-None ladder is a valid default for any T.
impl<T> Default for LayeredValue<T> {
    fn default() -> Self {
        LayeredValue {
            city: None,
            building: None,
            resident: None,
        }
    }
}

impl<T> LayeredValue<T> {
    pub fn resolve(&self) -> Option<&T> {
        self.resident
            .as_ref()
            .or(self.building.as_ref())
            .or(self.city.as_ref())
    }
}

/// One concern timezone: an already-resolved offset, never a tz name —
/// re-resolving names against a moving tz database would fork replayed
/// history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockZone {
    pub id: String,
    pub offset_min: i32,
}

/// What a run's execution boundary allows. Resolved as one value rather
/// than field by field: a layer that speaks about the sandbox speaks
/// about all of it, so an under-specified layer can only ever reduce
/// what a run may do, never silently grant something the layer above it
/// never mentioned.
///
/// Host facts are deliberately absent — where the Python artifact lives
/// and which shell binary exists belong to the machine, not to the city,
/// and a city carried to another machine must not carry its paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxLimits {
    /// Whether the shell arm may be offered at all. Off by default: a
    /// shell line is the one arm whose reach cannot be read off its
    /// arguments.
    pub shell: bool,
    /// Instruction budget for one sandboxed call.
    pub fuel: u64,
    /// Extra readable paths, relative to the city root. The write domain
    /// is decided elsewhere; this only widens what may be read.
    pub mounts: Vec<Address>,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        SandboxLimits {
            shell: false,
            fuel: crate::consts_policy::SANDBOX_FUEL_DEFAULT,
            mounts: Vec::new(),
        }
    }
}

/// One external tool server this run may reach, as its configuration
/// states it: a label, a program on this machine, and that program's
/// arguments.
///
/// The command and its arguments are host facts, for the same reason
/// the sandbox keeps them out: a city carried to another machine must
/// not carry this machine's paths inside its history. They live in
/// `CONFIG.toml` and never in a ledger payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServer {
    pub label: ServerLabel,
    pub transport: McpTransport,
}

/// How this city reaches one server. Exhaustive rather than a URL that
/// might also be a command: the two are different machines to start
/// talking to, they fail differently, and a configuration that leaves it
/// to be guessed is one that guesses wrong on the day it matters.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    /// A program on this machine, spoken to over its own pipes.
    Stdio { command: String, args: Vec<String> },
    /// A server reached over HTTP, which is how a hosted catalogue is
    /// published. The city holds no account with it: whatever it needs
    /// travels in the header the configuration names.
    Http {
        url: String,
        /// A header to send, as `name: value`. Absent for a server that
        /// asks for none.
        header: Option<String>,
    },
}

/// Frozen at Run start, never re-read within the Run.
/// Fields only grow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenConfig {
    pub clock_stamp: ClockStampGranularity,
    pub clock_zones: Vec<ClockZone>,
    /// Frozen for the same reason the tool table is: what a run may
    /// reach decides what it was told it could do, and a capability that
    /// widens mid-run is one nobody reviewed.
    pub sandbox: SandboxLimits,
    /// External tool servers, frozen for the same reason as the tool
    /// table itself: their tools enter the catalog at run start, a
    /// provider hashes that array ahead of the system prompt, and a
    /// table that widened mid-run would both discard the cache and
    /// hand the model a capability nobody reviewed.
    pub mcp: Vec<McpServer>,
    /// How hard the model may think. Frozen rather than live because the
    /// provider renders it into the cached prompt prefix: changing the
    /// effort value mid-run invalidates the message cache breakpoints,
    /// so a run that could retune itself would keep paying to rebuild
    /// the cache it just discarded. A change applies to the next run.
    pub effort: Option<Effort>,
}

/// Hot-reloadable surface. Empty in S2 by design: the type exists so the
/// no-field-overlap assertion guards every future addition (S4 fills it).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveConfig {}

/// Resolves the ladder into the Run-start snapshot. Absence everywhere
/// falls back to the policy default; the zones ladder overrides as a
/// whole list (a building that writes zones replaces the city list).
pub fn freeze(
    clock_stamp: &LayeredValue<ClockStampGranularity>,
    clock_zones: &LayeredValue<Vec<ClockZone>>,
    effort: &LayeredValue<Effort>,
    sandbox: &LayeredValue<SandboxLimits>,
    mcp: &LayeredValue<Vec<McpServer>>,
) -> FrozenConfig {
    FrozenConfig {
        clock_stamp: *clock_stamp.resolve().unwrap_or(&CLOCK_STAMP_DEFAULT),
        clock_zones: clock_zones.resolve().cloned().unwrap_or_default(),
        sandbox: sandbox.resolve().cloned().unwrap_or_default(),
        effort: effort.resolve().copied(),
        // A layer that speaks about servers speaks about all of them:
        // an unstated layer reaches nothing rather than inheriting a
        // reach nobody at that layer wrote down.
        mcp: mcp.resolve().cloned().unwrap_or_default(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
