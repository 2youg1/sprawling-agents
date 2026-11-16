// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Deserializer, Serialize};

use crate::discard::Restoration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileDiscarded {
    pub paths: Vec<String>,

    #[serde(
        default,
        deserialize_with = "restoration_if_named",
        skip_serializing_if = "Option::is_none"
    )]
    pub restoration: Option<Restoration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardRestored {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssetArchived {
    #[serde(default = "fact")]
    pub kind: String,

    #[serde(default)]
    pub day: u64,
    #[serde(default)]
    pub subject: String,
}

fn fact() -> String {
    "fact".to_owned()
}

fn restoration_if_named<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Restoration>, D::Error> {
    let raw = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(raw).ok())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
