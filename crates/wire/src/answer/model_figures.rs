// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The window and the ceiling the city would register for each row of a
//! model table the person left empty, each with the rung that answered
//! (`crates/wire/spec/Answer/ModelFigures.lean` §8-97, wire D58).
//!
//! Read shapes only: the ladder is `gateway::Figures::resolve`, the one
//! the pick point calls, so a page that draws these draws what a pick
//! with empty boxes registers rather than a second guess at it.

use kernel::{Ceiling, ModelTag, Window};
use serde::{Deserialize, Serialize};

/// One row of the table, as the page asks about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FiguresAsked {
    pub id: String,
    /// The window the provider's own list stated for this row; absent
    /// for a row the person named and the list did not carry.
    pub context_tokens: Option<Window>,
    /// The ceiling the provider's own list stated for this row.
    pub max_output_tokens: Option<Ceiling>,
    /// The role this row is to fill: the earlier registration of the
    /// same model under this tag is a rung of its own.
    pub tag: Option<ModelTag>,
}

/// Every row asked about, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelFiguresAnswer {
    pub models: Vec<FiguresLine>,
}

/// What one row would be registered with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FiguresLine {
    pub id: String,
    /// Absent when no rung states a window: the window is unknown and
    /// the context reminder stays silent.
    pub window: Option<WindowLine>,
    /// Absent only when the policy default itself is zero, a state the
    /// policy module's own test forbids.
    pub ceiling: Option<CeilingLine>,
}

/// A window and the rung that stated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WindowLine {
    pub tokens: Window,
    pub from: FigureFrom,
}

/// The ceiling a call would be made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CeilingLine {
    /// A figure every request states, and who stated it.
    Sent { tokens: Ceiling, from: FigureFrom },
    /// Nobody stated one on a face that takes none: the request leaves
    /// the field out and the provider applies its own default.
    ProviderDefault,
}

/// Who stated a figure, spelled as `model_selected` records it
/// (`window_from`, `ceiling_from`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum FigureFrom {
    /// A figure the person entered.
    Person,
    /// The provider's own model list.
    Upstream,
    /// What this tag was registered with when it last pointed at this
    /// same model.
    Registered,
    /// The pinned catalogue or the preset table.
    Preset,
    /// The city's policy default; never the source of a window.
    Policy,
}
