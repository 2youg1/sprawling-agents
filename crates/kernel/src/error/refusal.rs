// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate refusals: the three mandatory parts.

use serde::{Deserialize, Serialize};

/// The three mandatory parts of a gate refusal: rule | violation |
/// compliant alternative (three-part refusal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GateRefusal {
    rule: String,
    violation: String,
    alternative: String,
}

impl GateRefusal {
    pub fn new(
        rule: impl Into<String>,
        violation: impl Into<String>,
        alternative: impl Into<String>,
    ) -> Self {
        GateRefusal {
            rule: rule.into(),
            violation: violation.into(),
            alternative: alternative.into(),
        }
    }

    pub fn rule(&self) -> &str {
        &self.rule
    }

    pub fn violation(&self) -> &str {
        &self.violation
    }

    pub fn alternative(&self) -> &str {
        &self.alternative
    }
}
