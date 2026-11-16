// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Interpreter {
    #[default]
    System,
    Pwsh,
}

impl Interpreter {

    pub const KEY: &'static str = "interpreter";

    pub fn parse(raw: &str) -> Result<Interpreter, AxError> {
        match raw {
            "system" => Ok(Interpreter::System),
            "pwsh" => Ok(Interpreter::Pwsh),
            _ => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read a configuration layer",
                format!("[sandbox] {}: {raw}", Interpreter::KEY),
            )
            .with_recovery(
                "write `interpreter = \"system\"` for this platform's shell or \
                 `interpreter = \"pwsh\"` for PowerShell 7",
            )),
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Interpreter::System => "system",
            Interpreter::Pwsh => "pwsh",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::Interpreter;
    use crate::error::AxCode;

    #[test]
    fn two_spellings_are_read_and_any_other_is_refused_with_both_named() {
        for interpreter in [Interpreter::System, Interpreter::Pwsh] {
            assert_eq!(
                Interpreter::parse(interpreter.as_str()).unwrap(),
                interpreter
            );
        }
        let err = Interpreter::parse("bash").unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(err.subject().contains("interpreter") && err.subject().contains("bash"));
        assert_eq!(
            err.recovery(),
            "write `interpreter = \"system\"` for this platform's shell or `interpreter = \"pwsh\"` for PowerShell 7"
        );
    }
}
