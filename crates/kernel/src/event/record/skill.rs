// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What shelving a skill and auditing it record
//! (`crates/kernel/spec/Event/Record.lean` D23).
//!
//! The line is record-only: what an auditor said about a skill decides
//! no byte of a model request. A skill's use is not recorded here; the
//! `tool_called` lines that read it already say so.

use serde::{Deserialize, Serialize};

use crate::B3Hash;

/// `skill_audited`: one audit of one version of a skill.
///
/// The digest binds the verdict to the bytes it judged: once the skill's
/// content changes, its digest changes and this line no longer speaks
/// for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillAudited {
    pub skill: String,
    pub digest: B3Hash,
    pub source: AuditSource,
    /// The auditor's own version or name; for skills.sh, the partner,
    /// such as `socket`.
    pub scanner: String,
    pub verdict: AuditVerdict,
    /// The risk level in the auditor's own word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    /// When the auditor says it audited, in its own spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audited_at: Option<String>,
    /// Where a person reads the audit when the city could not fetch it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// `skill_shelved`: one version of a skill landed on a shelf through the
/// city's install door.
///
/// Written only when bytes actually landed: a reinstall of the same
/// content changes nothing and writes nothing. `digest` is the value the
/// audit binds to, so a version, its shelving and its audits meet on one
/// digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillShelved {
    pub skill: String,
    pub digest: B3Hash,
    pub source: ShelvedFrom,
}

/// Where a shelved version came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ShelvedFrom {
    /// A directory or document on this machine.
    Path,
    /// A clone of an `https://` repository at a revision.
    Git { url: String, rev: Option<String> },
    /// A skill skills.sh names as `<owner>/<repo>/<skill>`.
    SkillsSh { name: String },
    /// One of the skills the binary carries.
    Shipped,
    /// Text written on the shelf page.
    Page,
}

/// Who audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AuditSource {
    /// The partner audits skills.sh publishes for a skill.
    SkillsSh,
    /// The SkillSpector scanner run on this machine.
    SkillSpector,
}

/// What the audit concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AuditVerdict {
    Pass,
    Warn,
    Fail,
    /// An auditor applied and no audit could be had: skills.sh did not
    /// answer or refused, or SkillSpector exited with neither verdict code.
    /// Shelving goes ahead, and the audit state never counts this line as
    /// an audit. When no auditor applies - a local or shipped skill and no
    /// SkillSpector on the PATH - no line is written at all (city D19).
    Unreachable,
}
