// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "report", rename_all = "snake_case")]
pub enum HarnessReported {

    Said {

        text: String,
    },

    Thought {

        text: String,
    },

    ToolCall {

        call: String,

        title: String,

        kind: String,
    },

    ToolCallStatus {

        call: String,

        status: String,
    },

    Other {

        variant: String,
    },

    PermissionAsked {

        title: String,

        options: Vec<HarnessPermit>,
    },

    PermissionAnswered {

        chosen: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessPermit {

    pub id: String,

    pub name: String,

    pub kind: HarnessPermitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HarnessPermitKind {

    AllowOnce,

    AllowAlways,

    RejectOnce,

    RejectAlways,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessAnswered {

    pub stop: HarnessStop,

    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HarnessStop {

    EndTurn,

    MaxTokens,

    MaxTurnRequests,

    Refusal,

    Cancelled,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
