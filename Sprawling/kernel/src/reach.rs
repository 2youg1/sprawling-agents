// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Named {

    Resolved(u32),

    NotFound,

    Refused(String),

    ProxiedAway,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Connected {
    Open,

    Refused,

    Silent,
    Failed(String),

    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Answered {

    Status(u16),

    NameNotUsable(String),

    HandshakeFailed(String),

    Unreachable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Through {

    Direct,

    Environment(String),

    Excluded,

    LocalAddress,

    Disabled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Proxying {

    #[default]
    ExceptLocal,

    Always,

    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Reach {
    pub host: String,
    pub named: Named,
    pub connected: Connected,
    pub answered: Answered,
    pub through: Through,

    pub elapsed_ms: u64,
}

impl Reach {

    pub fn answered_at_all(&self) -> bool {
        matches!(self.answered, Answered::Status(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_status_is_an_answer_and_nothing_else_is() {
        let reading = |answered: Answered| Reach {
            host: "api.example.test".to_owned(),
            named: Named::Resolved(1),
            connected: Connected::Open,
            answered,
            through: Through::Direct,
            elapsed_ms: 40,
        };
        let verdicts: Vec<bool> = [
            Answered::Status(200),
            Answered::Status(401),
            Answered::NameNotUsable("a_b".to_owned()),
            Answered::HandshakeFailed("certificate".to_owned()),
            Answered::Unreachable("timed out".to_owned()),
        ]
        .into_iter()
        .map(|answered| reading(answered).answered_at_all())
        .collect();
        assert_eq!(verdicts, [true, true, false, false, false]);
    }
}
