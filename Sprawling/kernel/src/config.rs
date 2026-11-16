// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::consts_policy::CLOCK_STAMP_DEFAULT;
use crate::error::{AxCode, AxError};
use crate::locator::B3Hash;
use crate::model::Effort;
use crate::tool::ServerLabel;

mod connectors;
mod container;
mod exposure;
mod interpreter;
mod sandbox;
mod search;
pub use connectors::{McpDefinition, McpLayer, McpReference, definition_digest};
pub use container::{ContainerImage, ContainerLimits};
pub use exposure::{CatalogExposure, Exposure, ExposureFrom, ExposureTable};
pub use interpreter::Interpreter;
pub use sandbox::{SandboxArm, SandboxLimits};
pub use search::{SearchConfiguration, SearchSupplier};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EnvVarName(String);

impl EnvVarName {

    pub fn parse(raw: &str) -> Result<EnvVarName, AxError> {
        let reject = |violation: &str| {
            Err(AxError::failure(
                AxCode::InvalidArgs,
                "declare an environment variable",
                raw.to_owned(),
            )
            .with_recovery(format!(
                "{violation}; declare only the names a build needs, and keep a credential in \
                 the vault as a `secret:realm/name` reference"
            )))
        };
        if raw.is_empty() {
            return reject("the name is empty");
        }
        if raw.contains('=') {
            return reject("the name carries `=`, which separates a name from its value");
        }
        if raw.chars().any(char::is_control) {
            return reject("the name carries a control character");
        }
        if crate::secret::names_a_credential(raw) {
            return reject("the name reads as a credential");
        }
        Ok(EnvVarName(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EnvVarName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for EnvVarName {
    type Error = AxError;

    fn try_from(raw: String) -> Result<EnvVarName, AxError> {
        EnvVarName::parse(&raw)
    }
}

impl From<EnvVarName> for String {
    fn from(name: EnvVarName) -> String {
        name.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockStampGranularity {
    Off,
    Minute,
    FiveMinute,
    Hour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct SecondThreshold(u64);

impl SecondThreshold {

    pub fn parse(raw: u64) -> Result<SecondThreshold, AxError> {
        let floor = crate::consts_policy::CTX_REMINDER_SECOND_MIN;
        let ceiling = crate::consts_policy::CTX_REMINDER_SECOND_MAX;
        if (floor..=ceiling).contains(&raw) {
            return Ok(SecondThreshold(raw));
        }
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "set the second context-reminder threshold",
            raw.to_string(),
        )
        .with_recovery(format!(
            "the legal domain is {floor} through {ceiling} percent of the window; state a whole \
             percent in it, or leave the key out for {}",
            crate::consts_policy::CTX_REMINDER_SECOND_DEFAULT
        )))
    }

    #[must_use]
    pub fn percent(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for SecondThreshold {
    type Error = AxError;

    fn try_from(raw: u64) -> Result<SecondThreshold, AxError> {
        SecondThreshold::parse(raw)
    }
}

impl From<SecondThreshold> for u64 {
    fn from(rung: SecondThreshold) -> u64 {
        rung.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct ContextLock(u64);

impl ContextLock {

    pub fn parse(raw: u64) -> Result<ContextLock, AxError> {
        if raw == 0 {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "set a building context lock",
                "0".to_owned(),
            )
            .with_recovery(
                "state a positive whole number of input tokens, or leave `context_lock_tokens` \
                 out for reminder-only behaviour",
            ));
        }
        Ok(ContextLock(raw))
    }

    #[must_use]
    pub fn tokens(self) -> crate::Tokens {
        crate::Tokens::new(self.0)
    }

    #[must_use]
    pub fn effective(self, model_window: crate::Tokens) -> crate::Tokens {
        crate::Tokens::new(self.0.min(model_window.get()))
    }
}

impl TryFrom<u64> for ContextLock {
    type Error = AxError;

    fn try_from(raw: u64) -> Result<ContextLock, AxError> {
        ContextLock::parse(raw)
    }
}

impl From<ContextLock> for u64 {
    fn from(lock: ContextLock) -> u64 {
        lock.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredValue<T> {
    pub city: Option<T>,
    pub building: Option<T>,
    pub resident: Option<T>,
}

impl<T> Default for LayeredValue<T> {
    fn default() -> Self {
        LayeredValue {
            city: None,
            building: None,
            resident: None,
        }
    }
}

impl<T> LayeredValue<T> {
    pub fn resolve(&self) -> Option<&T> {
        self.resident
            .as_ref()
            .or(self.building.as_ref())
            .or(self.city.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockZone {
    pub id: String,
    pub offset_min: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServer {
    pub label: ServerLabel,
    pub transport: McpTransport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum McpTransport {

    Stdio {
        command: String,
        args: Vec<String>,

        env: Vec<(String, String)>,
    },

    Http {
        url: String,

        headers: Vec<(String, String)>,
    },

    Sse {
        url: String,

        headers: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenConfig {
    pub clock_stamp: ClockStampGranularity,
    pub clock_zones: Vec<ClockZone>,

    pub sandbox: SandboxLimits,

    pub mcp: Vec<McpServer>,

    #[serde(default)]
    pub mcp_references: Vec<McpReference>,

    #[serde(default)]
    pub skill_pins: std::collections::BTreeMap<String, B3Hash>,

    pub effort: Option<Effort>,

    pub second_threshold: Option<SecondThreshold>,

    #[serde(default)]
    pub search: SearchConfiguration,

    #[serde(default)]
    pub catalog: CatalogExposure,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatedConfig {
    pub clock_stamp: LayeredValue<ClockStampGranularity>,
    pub clock_zones: LayeredValue<Vec<ClockZone>>,
    pub effort: LayeredValue<Effort>,
    pub sandbox: LayeredValue<SandboxLimits>,
    pub mcp: LayeredValue<McpLayer>,
    pub skill_pins: LayeredValue<std::collections::BTreeMap<String, B3Hash>>,
    pub second_threshold: LayeredValue<SecondThreshold>,
    pub search: LayeredValue<SearchConfiguration>,
    pub catalog: LayeredValue<ExposureTable>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveConfig {}

pub fn freeze(stated: &StatedConfig) -> Result<FrozenConfig, AxError> {
    let (mcp, mcp_references) = connectors::resolve(&stated.mcp)?;
    Ok(FrozenConfig {
        clock_stamp: *stated.clock_stamp.resolve().unwrap_or(&CLOCK_STAMP_DEFAULT),
        clock_zones: stated.clock_zones.resolve().cloned().unwrap_or_default(),
        sandbox: stated.sandbox.resolve().cloned().unwrap_or_default(),
        effort: stated.effort.resolve().copied(),
        mcp,
        mcp_references,
        skill_pins: stated.skill_pins.resolve().cloned().unwrap_or_default(),
        second_threshold: stated.second_threshold.resolve().copied(),

        search: stated.search.resolve().cloned().unwrap_or_default(),

        catalog: CatalogExposure {
            city: stated.catalog.city.clone().unwrap_or_default(),
            building: stated.catalog.building.clone().unwrap_or_default(),
        },
    })
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
