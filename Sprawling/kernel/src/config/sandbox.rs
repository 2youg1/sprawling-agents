// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{ContainerLimits, EnvVarName, Interpreter};
use crate::{Address, AxCode, AxError, ServerLabel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SandboxArm {
    None,
    CopiedTree,
    Native,
    Container,
    Python,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SandboxLimits {

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm: Option<SandboxArm>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerLimits>,

    pub shell: bool,

    #[serde(default)]
    pub interpreter: Interpreter,

    pub fuel: u64,

    pub mounts: Vec<Address>,

    #[serde(default)]
    pub env_passthrough: Vec<EnvVarName>,

    #[serde(default)]
    pub trusted: Vec<ServerLabel>,
}

impl SandboxLimits {

    pub fn selected_arm(&self) -> Result<Option<SandboxArm>, AxError> {
        match (self.arm, self.container.as_ref()) {
            (Some(SandboxArm::Container), None) => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "choose the container arm",
                "container limits are missing",
            )
            .with_recovery("declare every limit in [sandbox.container]")),
            (
                Some(
                    SandboxArm::None
                    | SandboxArm::CopiedTree
                    | SandboxArm::Native
                    | SandboxArm::Python,
                ),
                Some(_),
            ) => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "choose the sandbox arm",
                "container limits conflict with the explicit arm",
            )
            .with_recovery("select container or remove the container limits")),
            (Some(arm), _) => Ok(Some(arm)),
            (None, Some(_)) => Ok(Some(SandboxArm::Container)),
            (None, None) => Ok(None),
        }
    }

    #[must_use]
    pub fn trusts(&self, label: &ServerLabel) -> bool {
        self.trusted.iter().any(|allowed| allowed == label)
    }
}

impl Default for SandboxLimits {
    fn default() -> Self {
        SandboxLimits {
            container: None,
            arm: None,
            shell: false,
            interpreter: Interpreter::System,
            fuel: crate::consts_policy::SANDBOX_FUEL_DEFAULT,
            mounts: Vec::new(),
            env_passthrough: Vec::new(),
            trusted: Vec::new(),
        }
    }
}
