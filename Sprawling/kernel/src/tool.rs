// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::error::{AxCode, AxError};
use crate::event::Payload;

pub mod writes;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolName(String);

impl ToolName {

    pub const BROWSER: &'static str = "browser";

    pub const USER_BROWSER: &'static str = "usersbrowser";

    pub const EXEC: &'static str = "exec";

    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let well_formed = !raw.is_empty()
            && raw
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if well_formed {
            Ok(ToolName(raw.to_owned()))
        } else {
            Err(
                AxError::failure(AxCode::InvalidArgs, "parse tool name", raw)
                    .with_recovery("use ascii lowercase, digits and underscore, non-empty"),
            )
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReservedNamespace {

    Computer,

    Media,

    Record,

    Skill,

    Sprawling,

    Web,
}

impl ReservedNamespace {

    pub const ALL: [ReservedNamespace; 6] = [
        ReservedNamespace::Computer,
        ReservedNamespace::Media,
        ReservedNamespace::Record,
        ReservedNamespace::Skill,
        ReservedNamespace::Sprawling,
        ReservedNamespace::Web,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ReservedNamespace::Computer => "computer",
            ReservedNamespace::Media => "media",
            ReservedNamespace::Record => "record",
            ReservedNamespace::Skill => "skill",
            ReservedNamespace::Sprawling => "sprawling",
            ReservedNamespace::Web => "web",
        }
    }

    #[must_use]
    pub fn named(raw: &str) -> Option<ReservedNamespace> {
        Self::ALL
            .into_iter()
            .find(|namespace| namespace.as_str() == raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServerLabel(String);

impl ServerLabel {

    pub fn parse(raw: &str) -> Result<ServerLabel, AxError> {
        let well_formed = !raw.is_empty()
            && raw
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        if well_formed {
            Ok(ServerLabel(raw.to_owned()))
        } else {
            Err(
                AxError::failure(AxCode::InvalidArgs, "name an mcp server", raw.to_owned())
                    .with_recovery("use ascii lowercase and digits, non-empty"),
            )
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServerLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ServerLabel {
    type Error = AxError;

    fn try_from(raw: String) -> Result<ServerLabel, AxError> {
        ServerLabel::parse(&raw)
    }
}

impl From<ServerLabel> for String {
    fn from(label: ServerLabel) -> String {
        label.0
    }
}

impl Serialize for ToolName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ToolName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        ToolName::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutMs(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Effect {
    Read,
    Write {
        domain: Address,
    },

    Egress,

    Connector {
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        label: ServerLabel,
    },

    Spawn,

    Govern,

    AttachUserBrowser {
        address: Option<String>,
    },

    Spend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Temporal {
    Timeless,
    Timestamped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostTier {
    Free,
    Light,
    Heavy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RenderIntent {
    Generic,
    Terminal,
    Diff {
        locations: Vec<Address>,
    },

    Signal,

    Delegate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMeta {
    pub name: ToolName,
    pub disclosure: String,
    pub params: Payload,
    pub effect: Effect,
    pub cost_tier: CostTier,
    pub timeout: Option<TimeoutMs>,
    pub render: RenderIntent,
    pub temporal: Temporal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {

    pub id: String,
    pub name: ToolName,
    pub args: Payload,
}

impl ToolCall {

    pub fn action(&self) -> Result<Vec<u8>, AxError> {
        let mut action = self.name.as_str().as_bytes().to_vec();
        let args = serde_json::to_string(&self.args).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a tool call's action",
                self.name.as_str().to_owned(),
            )
            .with_recovery(format!(
                "JSON refused these arguments ({err}); call the tool again with whole \
                 numbers and strings, which is all this city's payloads carry"
            ))
        })?;
        action.extend_from_slice(args.as_bytes());
        Ok(action)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutcome {
    pub result: Payload,

    #[serde(default)]
    pub attachments: Vec<crate::ImageRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateSubject {

    Area(Address),

    Room(Address),

    Scope(String),

    Host(String),

    None,
}

pub trait Tool: Send + Sync {
    fn meta(&self) -> &ToolMeta;

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError>;

    fn subject(&self, _call: &ToolCall) -> Result<GateSubject, AxError> {
        Ok(GateSubject::None)
    }

    fn effect_of(&self, _call: &ToolCall) -> Result<Effect, AxError> {
        Ok(self.meta().effect.clone())
    }

    fn writes(&self, _call: &ToolCall) -> writes::Writes {
        writes::Writes::of(&self.meta().effect)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecArm {
    Program { path: String, args: Vec<String> },
    Python { code: String },
    Shell { text: String },
}

#[cfg(feature = "conformance")]
pub mod conformance;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
