// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How far the person has got through the first-run guide, kept per
//! city (wire-SPEC.md 8-68).
//!
//! What this records is where the person went, not what is configured:
//! whether a step is done is read from the configuration and the doctor
//! each time, and a step opened or skipped here is never drawn as done.

use serde::{Deserialize, Serialize};

/// The guide's progress in one city: where it reopens, whether the
/// person has left it, and each optional step seen or put off.
///
/// The provider step has no mark: it is the one step that must be done,
/// and only the city's saved endpoint and `main` model say whether it is.
/// Every field defaults, so a city nobody has guided reads as a guide at
/// its start.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GuideProgress {
    /// The step the guide opens at next; absent until the person moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<GuideStep>,
    #[serde(default)]
    pub state: GuideState,
    /// Step 2, the dependencies the doctor reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<GuideMark>,
    /// Step 3, the texts and the names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texts: Option<GuideMark>,
    /// Step 4, importing a skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<GuideMark>,
    /// Step 5, connecting a tool server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<GuideMark>,
}

/// The five steps, in the order the guide gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GuideStep {
    Provider,
    Dependencies,
    Texts,
    Skills,
    Mcp,
}

/// Whether opening the city still offers the guide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GuideState {
    /// The guide is offered when the city opens.
    #[default]
    Open,
    /// The person left it: the city opens on the conversation, and the
    /// guide opens from the settings.
    Left,
}

/// What the person did with one optional step. A step with no mark has
/// not been looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GuideMark {
    Seen,
    Skipped,
}
