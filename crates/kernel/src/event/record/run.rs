// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a dispatch and a fork record about a run.

use serde::{Deserialize, Serialize};

use crate::event::identity::{RunId, Seq};
use crate::locator::{B3Hash, Locator};

/// One skill a run was dispatched with, pinned to the bytes it read.
///
/// The hash is the point: a page answers "which runs read these exact
/// bytes" from it, and a pin naming a name alone cannot answer that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillPin {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
}

/// `run_started`: the work a run was given, and where it came from.
///
/// Every field defaults, because the golden fixtures hold a
/// `run_started` whose payload is `{}` and a projection that refused it
/// would refuse a ledger this repository ships.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunStarted {
    /// What the person asked for, in their words.
    #[serde(default)]
    pub task: String,
    /// What finishing looks like, in their words.
    #[serde(default)]
    pub goal: String,
    /// The job this dispatch came out of. Absent only in a record
    /// written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub job: Option<Locator>,
    /// The run that forked this one, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<RunId>,
    /// The run this one replaced, when it is a successor. Spelled the
    /// same way here and on `checkpoint_committed`, because it is one
    /// fact: `memory::checkpoint::provenance` used to own a second
    /// spelling of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<RunId>,
    /// Written even when empty, and deliberately: a key that comes and
    /// goes is a shape a reader has to guess at, and "this building
    /// admits no skills" is a fact worth recording rather than an
    /// absence to infer.
    #[serde(default)]
    pub skills: Vec<SkillPin>,
}

/// `run_forked`: which run this one continues, and from where in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunForked {
    /// The run whose history this one resumes.
    pub from: RunId,
    /// The last sequence number of `from` that this run inherits.
    pub at_seq: Seq,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
