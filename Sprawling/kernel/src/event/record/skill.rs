// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::B3Hash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillAudited {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_only_reason: Option<String>,
    pub skill: String,
    pub digest: B3Hash,
    pub source: AuditSource,

    pub scanner: String,
    pub verdict: AuditVerdict,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audited_at: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillShelved {
    pub skill: String,
    pub digest: B3Hash,
    pub source: ShelvedFrom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ShelvedFrom {
    Path,

    Git { url: String, rev: Option<String> },

    SkillsSh { name: String },

    Shipped,

    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AuditSource {
    SkillsSh,

    SkillSpector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AuditVerdict {
    Pass,
    Warn,
    Fail,

    Unreachable,
}
