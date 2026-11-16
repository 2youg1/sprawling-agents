// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use serde::{Deserialize, Serialize};

use crate::carried_name::ToolkitSlug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ToolkitsAnswer {
    Unenrolled,

    Shelf { toolkits: Vec<ToolkitLine> },

    Refused { refusal: Box<AxError> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolkitLine {
    pub slug: ToolkitSlug,

    pub name: String,

    pub auth: String,
    pub standing: Standing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Standing {
    Absent,

    Awaiting { consent_url: String },

    Connected { alias: String },

    Refused { refusal: Box<AxError> },
}
