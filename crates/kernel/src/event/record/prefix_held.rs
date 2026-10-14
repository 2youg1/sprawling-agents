// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One wait a model call took at its endpoint's gate behind a pre-warm
//! of the prefix a building shares, so a person can read off the history
//! why a call went out late and what the pre-warm cost
//! (`crates/kernel/spec/Event/Record.lean` D60, gateway D43).
//!
//! The pre-warm's request is absent for the reason `ModelCalled` carries
//! no body: its bytes are the shared part of the request that waited,
//! which `prompt_assembled` already recorded.

use serde::{Deserialize, Serialize};

use crate::budget::UsdMicros;
use crate::error::AxError;
use crate::model::ModelUsage;

/// `prefix_held`: one attempt's wait at its endpoint's gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixHeld {
    /// Whole microseconds from reaching the gate to being let on toward
    /// a ticket, on the gate's injected monotonic clock.
    pub waited_us: u64,
    /// How the pre-warm this call waited for ended.
    pub ended: PrewarmEnd,
    /// What the pre-warm cost or why it was refused; only on the line of
    /// the call that sent it, so one request is counted once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent: Option<PrewarmSent>,
}

/// How one pre-warm ended, as every call that waited for it saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PrewarmEnd {
    /// The provider answered it.
    Answered,
    /// The provider refused it, or it failed on the way.
    Refused,
    /// `HOLD_MAX` passed before it came back, or it found no permit in
    /// time.
    TimedOut,
    /// The thread that sent it stopped before it came back.
    Abandoned,
}

/// The pre-warm's own answer, on the line of the call that sent it.
///
/// `Refused` is tried first when reading, as in `CacheRenewed`: every
/// field of `Answered` may be absent, so it would read any line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum PrewarmSent {
    /// The failure the pre-warm came back with, unchanged.
    Refused {
        /// The failure the provider or the gate returned.
        refused: AxError,
    },
    /// The provider answered the pre-warm.
    Answered {
        /// Four token counts, as the provider reported them. Absent when
        /// it reported none, which is not the same fact as zero.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(
            feature = "schema",
            schemars(with = "Option<std::collections::BTreeMap<String, u64>>")
        )]
        usage: Option<ModelUsage>,
        /// What the provider itself billed, when it returns a figure.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        billed_usd_micros: Option<UsdMicros>,
    },
}
