// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one address is actually governed by, and which file said so.

use kernel::{Address, Effort, Proxying};
use serde::{Deserialize, Serialize};

/// Which file settled one value.
///
/// **Every value in a [`ConfigAnswer`] carries one.** A settings page
/// that was told only the resolved figure could not say whether the
/// person is looking at their own entry, at something the building
/// inherited, or at the city's — so pressing "reset" and pressing
/// nothing looked the same, and the page had to climb the ladder a
/// second time to find out. This is that answer, stated once by the
/// layer that resolved it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ConfigLayer {
    /// The city's own `CONFIG.toml`.
    City,
    /// The building's file, which covers every room under it.
    Resident,
    /// The room's own file, which covers one session.
    Room,
}

/// `[model] effort`, and the file that settled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledEffort {
    pub effort: Effort,
    pub from: ConfigLayer,
}

/// What an endpoint that settled nothing is called with.
///
/// **These are the gateway's figures, read out rather than restated.**
/// The endpoint form used to ship its own three numbers as placeholder
/// text, so an endpoint attached through the form and one attached by
/// import behaved differently while the person had filled in nothing;
/// the form now draws what this carries, and the numbers have one home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TuningDefaults {
    /// How long one request may take.
    pub timeout_ms: u64,
    /// How many times a request that failed for a reason worth retrying
    /// is made again.
    pub request_max_retries: u32,
    /// How long a streamed answer may stay silent before the call is
    /// abandoned.
    pub stream_idle_timeout_ms: u64,
    /// Which calls go through the machine's proxy.
    pub proxying: Proxying,
}

/// What one address is governed by, value by value, with the file each
/// value came from.
///
/// Answered for an address rather than for the city, because the ladder
/// has three rungs and only an address says which room's rung is in
/// play. An address naming a building answers with two rungs climbed
/// and no room entry, which is what a building with no open session is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigAnswer {
    /// The address the ladder was climbed for.
    pub addr: Address,
    /// How hard the model is asked to think here. Absent means no
    /// layer stated it and the provider's own default answers, which
    /// is a statement this city deliberately makes rather than filling
    /// in a level nobody chose.
    pub effort: Option<SettledEffort>,
    pub tuning: TuningDefaults,
}
