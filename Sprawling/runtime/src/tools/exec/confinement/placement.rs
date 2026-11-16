// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {

    Sandbox,

    Host,
}

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
