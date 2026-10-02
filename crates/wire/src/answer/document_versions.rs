// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The versions of one document, newest first
//! (`crates/wire/spec/Answer/DocumentVersions.lean` §8-83).
//!
//! The history knows the versions a page wrote; a resident's `edit`, a
//! command or the person's own editor writes past the city's document
//! door and leaves no line. Those versions are named only where the city
//! saw them - on disk now, or as the version a recorded save was made
//! on - and the answer says no more about them than that.

use kernel::{Address, B3Hash, Seq, TimeMs};
use serde::{Deserialize, Serialize};

/// The most versions one answer lists.
pub const VERSIONS_MAX: usize = 100;

/// One document's versions, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VersionsAnswer {
    pub at: Address,
    pub versions: Vec<DocumentVersion>,
    /// Older versions were left out at [`VERSIONS_MAX`].
    pub more: bool,
}

/// One version, and where the city learnt of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentVersion {
    pub version: B3Hash,
    /// Its length, when a line or the content store says it.
    pub bytes: Option<u64>,
    /// The content store holds its bytes, so `Range`, `Bytes` and
    /// `Export` can read it.
    pub kept: bool,
    pub source: VersionSource,
}

/// Where a version came from, as far as the city can say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum VersionSource {
    /// On disk now, and written by no save the history recorded.
    OnDisk,
    /// Written through a page - a save, or proposal cards a person
    /// accepted - by the `document_written` line at `seq`.
    Saved { seq: Seq, at: TimeMs },
    /// The version the save at `seq` was made on, which no earlier
    /// recorded save wrote: written outside the page before it.
    Before { seq: Seq },
}
