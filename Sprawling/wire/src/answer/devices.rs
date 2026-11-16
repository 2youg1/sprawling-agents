// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The browsers paired at this machine's door
//! (`crates/wire/spec/Answer/Devices.lean` §8-91).

use kernel::TimeMs;
use serde::{Deserialize, Serialize};

/// Every paired browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DevicesAnswer {
    pub devices: Vec<DeviceLine>,
}

/// One paired browser, as a person recognises and revokes it. Its public
/// key and its sessions stay in the city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceLine {
    pub id: DeviceId,
    /// The name the page gave when it paired.
    pub label: String,
    pub paired_at: TimeMs,
    /// The last time it opened a session; absent until it has.
    pub last_seen: Option<TimeMs>,
}

/// A paired browser, by the id the city gave it when it paired.
///
/// The text as the city wrote it; the wire does not read it. An id the
/// city does not know and a misspelt one get the same refusal, because for
/// the door both mean there is no such device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceId(String);

impl DeviceId {
    /// The id as the city wrote it.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(text)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
