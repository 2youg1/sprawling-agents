// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Named command placement and the write-policy gate.

use kernel::{AxCode, AxError};
use serde_json::{Map, Value};

/// Where a host command runs. The safe arm is what a call gets when it
/// says nothing, because an agent that already knew a command was
/// dangerous would not need to be told to sandbox it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// In the confinement [`Confinement::detect`](super::Confinement::detect) reports: a copy of
    /// the working tree, and the platform's own isolation where there is
    /// a wrapper for it.
    Sandbox,
    /// Where it stands, on the room the run may write in. This is the
    /// placement a person decides on; a command that needs it asks for
    /// it by name.
    Host,
}

/// Reads the placement out of the call. Absent is [`Placement::Sandbox`].
pub fn parse_placement(args: &Map<String, Value>) -> Result<Placement, AxError> {
    match args.get("where") {
        None => Ok(Placement::Sandbox),
        Some(Value::String(said)) if said == "sandbox" => Ok(Placement::Sandbox),
        Some(Value::String(said)) if said == "host" => Ok(Placement::Host),
        Some(other) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            format!("unrecognised placement: {other}"),
        )
        .with_recovery(
            "leave `where` out to run in the sandbox, or name one of `sandbox` and \
             `host`",
        )),
    }
}

impl Placement {
    /// This placement, when the run's write limit opens it.
    ///
    /// A command on the host could change, remove or link any file it
    /// reaches, and nothing on this machine makes the existing ones
    /// read-only to it without administrator rights, so under `Create`
    /// only the copy is open; what a command writes there stays there
    /// (`crates/runtime/spec/Tools.lean` §8-55, runtime D11).
    ///
    /// # Errors
    /// The refusal of `kernel::gate::replacing`, naming the run's
    /// domain, before any process starts.
    pub fn opened_by(self, setup: &crate::tools::ExecSetup) -> Result<Placement, AxError> {
        match self {
            Placement::Sandbox => Ok(self),
            Placement::Host => {
                match kernel::gate::replacing(setup.policy.now().write, &setup.domain) {
                    kernel::GateOutcome::Allow => Ok(self),
                    kernel::GateOutcome::Deny { refusal } => Err(*refusal),
                    kernel::GateOutcome::Ask { question } => Err(*question),
                }
            }
        }
    }
}
