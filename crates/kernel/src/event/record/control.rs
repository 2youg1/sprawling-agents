// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city records about its own founding, about a run being
//! stopped or handed on, and about the governance lines no writer
//! records yet.
//!
//! Three kinds of this family carry a type that lives elsewhere, and
//! the struct is that type rather than a copy of it: `gate_denied` and
//! `budget_limit` carry an [`AxError`](crate::AxError) laid flat, the
//! carrier table in `kernel::error` being the authority on which code
//! lands under which kind; `approval_requested` carries an
//! [`ApprovalItem`](crate::ApprovalItem). `result_offloaded` stays with
//! `runtime::sieve`, whose account it flattens in.

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::locator::Locator;

/// `city_initialized`: the first line of every history. The city's name
/// is the record's own `addr`, so the payload holds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityInitialized {}

/// `building_created`: a building now stands at `addr`, laid out from
/// `template`, or found already standing when `adopted`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingCreated {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub addr: Address,
    /// The template's own name, as `city::BuildingTemplate::name`
    /// spells it.
    pub template: String,
    /// Written only on an adoption, as the hand-written writer left it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub adopted: bool,
}

/// `building_configured`: which faces of a building's own layer one
/// reconfiguration wrote. The values themselves stay in the files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingConfigured {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub addr: Address,
    pub sandbox: bool,
    pub mcp: bool,
    pub desktop: bool,
    pub context: bool,
}

/// `cancel_received`: the turn saw the stop. The run it belongs to is
/// the envelope's, so the payload holds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CancelReceived {}

/// `handoff_written`: the five sections a successor starts from.
///
/// `runtime::handoff::Handoff` is the only way to hold a valid one; this
/// is what it writes, so its refusal of an empty must-read list is not
/// re-checked by a reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HandoffWritten {
    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    pub must_read: Vec<Locator>,
    pub overview: String,
    pub progress: String,
    pub context: String,
    pub next_step: String,
}

/// `watchdog_fired`: what the watchdog did, and how often it has had to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WatchdogFired {
    #[serde(flatten)]
    pub action: FiredAction,
    /// How many corrective steers this run has been given.
    pub corrections: u32,
    /// How many provider failures this run has met.
    pub provider_failures: u32,
}

/// What the watchdog did, in the word the line carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FiredAction {
    /// The model was told it is repeating itself.
    Steer { text: String },
    /// The same call will be made again, no earlier than `until_ms`,
    /// because of the failure whose stable code and subject are named
    /// here.
    BackOff {
        until_ms: u64,
        code: String,
        subject: String,
    },
    /// The run was frozen.
    Freeze { reason: String },
}

/// `gate_checked`: a gate let an action through.
///
/// No production line records this kind, so no bytes fix its shape; the
/// struct is empty by decision, because the action it admitted is the
/// next line of the same run and a copy here would be a second spelling
/// of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GateChecked {}

/// `policy_created` and `policy_revoked`: a standing rule named `id`
/// came into force or left it.
///
/// No line records either kind yet, so no bytes fix the shape. The
/// struct holds the rule's name alone by decision: the rule's text is a
/// governed document, which `governed_document_written` already records,
/// and a copy here would be its second authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PolicyChanged {
    pub id: String,
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
