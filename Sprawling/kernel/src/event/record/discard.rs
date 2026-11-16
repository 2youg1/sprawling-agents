// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a building let go of: files a settlement found deleted, the
//! restoration of those files, and entries a run filed on the shelf.

use serde::{Deserialize, Deserializer, Serialize};

use crate::discard::Restoration;

/// `file_discarded`: the paths one sweep found deleted, and the way back.
///
/// `paths` stay the strings the writer spelled (`file:<path>`) rather
/// than [`Address`](crate::Address)es, because git keeps names an
/// address refuses and a discard of such a file is still a discard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileDiscarded {
    pub paths: Vec<String>,
    /// Absent, or spelled in a scheme this build cannot name, reads as
    /// `None` and the paths still read: a row without its way back is
    /// still a file that is gone.
    #[serde(
        default,
        deserialize_with = "restoration_if_named",
        skip_serializing_if = "Option::is_none"
    )]
    pub restoration: Option<Restoration>,
}

/// `discard_restored`: the paths a restoration put back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardRestored {
    pub paths: Vec<String>,
}

/// `asset_archived`: one entry a run filed in its building's archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssetArchived {
    /// The archive kind's word. A record that names none is a fact,
    /// the kind an unmarked entry has always been shown as.
    #[serde(default = "fact")]
    pub kind: String,
    /// The day the entry was filed, as the archive counts days.
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
