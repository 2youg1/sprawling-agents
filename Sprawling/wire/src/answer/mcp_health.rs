// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, Payload, ServerLabel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpHealthAnswer {
    pub addr: Address,
    pub servers: Vec<McpServerHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServerHealth {
    pub label: ServerLabel,

    pub transport: String,

    pub target: String,
    pub state: McpState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum McpState {

    Connected {

        protocol_version: String,

        server: String,
        tools: Vec<McpToolLine>,
    },

    Authenticating {

        recovery: String,
    },

    Failed { refusal: Box<AxError> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpToolLine {

    pub remote: String,

    pub name: String,
    pub disclosure: String,

    pub input_schema: Payload,
}
