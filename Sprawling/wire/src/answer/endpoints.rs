// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Ceiling, DialectKind, ModelTag, ServerLabel};
use serde::{Deserialize, Serialize};

use super::ModelFactsSummary;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointsAnswer {
    pub endpoints: Vec<EndpointSummary>,
    pub chosen: Vec<ChosenSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointSummary {
    pub name: String,

    pub label: String,
    pub base_url: String,
    pub dialect: DialectKind,

    pub connection_kind: String,

    pub models: Vec<ModelFactsSummary>,

    pub local: bool,
    pub has_credential: bool,

    pub tuning: crate::EndpointTuning,

    pub account_status: Vec<AccountStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountStatus {
    pub id: ServerLabel,
    pub key: KeyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum KeyState {

    Anonymous,

    Stored,

    Missing,

    Environment,

    EnvironmentUnusable,

    Unread,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChosenSummary {
    pub tag: ModelTag,
    pub endpoint: String,
    pub model: String,

    pub max_output_tokens: Option<Ceiling>,
}
