// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Frozen execution boundary and its explicit mechanism choice.

use super::{ContainerLimits, EnvVarName, Interpreter};
use crate::{Address, AxCode, AxError, ServerLabel};
use serde::{Deserialize, Serialize};

/// One sandbox mechanism family, shared by configuration and doctor reports.
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

/// What a run's execution boundary allows. Resolved as one value rather
/// than field by field: a layer that speaks about the sandbox speaks
/// about all of it, so an under-specified layer can only ever reduce
/// what a run may do, never silently grant something the layer above it
/// never mentioned.
///
/// Host facts are deliberately absent — where the Python artifact lives
/// and which shell binary exists belong to the machine, not to the city,
/// and a city carried to another machine must not carry its paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SandboxLimits {
    /// A named arm refuses when unavailable; absence preserves the platform default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm: Option<SandboxArm>,
    /// Explicit container policy; absence uses the existing platform confinement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerLimits>,
    /// Whether the shell arm may be offered at all. Off by default: a
    /// shell line is the one arm whose reach cannot be read off its
    /// arguments.
    pub shell: bool,
    /// Which interpreter the shell arm runs a line under: a name, never a path.
    #[serde(default)]
    pub interpreter: Interpreter,
    /// Instruction budget for one sandboxed call.
    pub fuel: u64,
    /// Extra readable paths, relative to the city root. The write domain
    /// is decided elsewhere; this only widens what may be read.
    pub mounts: Vec<Address>,
    /// Environment variable names a child process started by this scope
    /// may inherit, on top of the four every run gets.
    ///
    /// Declared name by name rather than widened once for everybody: a
    /// build chain that needs to find its linker needs a handful of
    /// names this machine happens to set, and a longer built-in list
    /// would hand every run in every city whatever those names hold.
    #[serde(default)]
    pub env_passthrough: Vec<EnvVarName>,
    /// Which connector servers this floor lets reach effects nothing
    /// in the city can take back — today, this machine's own desktop.
    ///
    /// Named server by server, and empty by default, for the reason
    /// `env_passthrough` is: a run that presses keys on somebody's
    /// keyboard is doing something no checkpoint puts back, so the floor
    /// says which connector may, in the one file a person edits.
    #[serde(default)]
    pub trusted: Vec<ServerLabel>,
}

impl SandboxLimits {
    /// Resolves the explicit sandbox choice, rejecting contradictory input.
    ///
    /// # Errors
    /// Refuses a selected container without limits or limits on another explicit arm.
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

    /// Whether this floor lets `label` reach what nothing here can
    /// undo. The one reader of the trusted list, so the answer cannot
    /// be spelled two ways.
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
