// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run left behind that somebody can check it by.
//!
//! **No bytes travel here.** A screenshot is a `cas:` locator and two
//! integer sides; fetching the picture is the asset endpoint's, for the
//! reason a whole batch of patch text is refused on a change list -
//! "show me what this run did" must not cost what every pixel it took
//! costs.

use kernel::{Locator, RunId, Seq};
use serde::{Deserialize, Serialize};

/// Which kind of evidence one row is.
///
/// Exhaustive and small: a row exists because the Ledger holds a
/// locator, and there are two records that write one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum EvidenceKind {
    /// A picture the browser tool put in the content store.
    Screenshot,
    /// The completion a plan node was closed with.
    Finished,
}

/// The two sides and the media type of a picture, as the record wrote
/// them. Absent on a row whose record named a locator and no size,
/// which is a row worth showing and not a size worth inventing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Picture {
    pub media_type: String,
    pub width: u32,
    pub height: u32,
}

/// One thing a run wrote down that can be looked at again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvidenceItem {
    /// Where in the Ledger the record sits, so a reader can go on from
    /// there.
    pub at: Seq,
    pub kind: EvidenceKind,
    pub locator: Locator,
    pub picture: Option<Picture>,
}

/// Everything one run left as evidence, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvidenceAnswer {
    pub run: RunId,
    pub items: Vec<EvidenceItem>,
}
