// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentCatalogAnswer {

    pub detected: Vec<AgentOffer>,

    pub catalog: Vec<AgentOffer>,

    pub added: Vec<AgentLine>,
    pub snapshot: CatalogSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogSnapshot {

    pub date: String,

    pub etag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentOffer {
    pub id: String,
    pub name: String,
    pub source: AgentSource,

    pub launch_preview: String,
    pub version: Option<String>,
    pub pinned: PinState,
    pub licence: Option<String>,

    pub env_names: Vec<String>,
    pub login: Vec<LoginKind>,

    pub spec_digest: B3Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AgentSource {

    Registry,

    Detected,

    Pasted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PinState {

    Exact,

    Floating,

    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginKind {

    Agent,

    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentLine {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    pub version: Option<String>,
    pub pinned: PinState,
    pub login_state: LoginState,

    pub auth_methods: Vec<AuthMethod>,

    pub seated_in: Vec<Address>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginState {

    Unasked,

    Ready,

    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuthMethod {
    pub id: String,
    pub name: String,
    pub kind: LoginKind,
}
