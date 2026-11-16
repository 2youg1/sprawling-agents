// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GuideProgress {

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<GuideStep>,
    #[serde(default)]
    pub state: GuideState,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<GuideMark>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texts: Option<GuideMark>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<GuideMark>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<GuideMark>,
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GuideState {

    #[default]
    Open,

    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GuideMark {
    Seen,
    Skipped,
}
