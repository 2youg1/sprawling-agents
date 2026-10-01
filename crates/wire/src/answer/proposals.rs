// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The proposal cards still open on one document (wire-SPEC.md 8-73).
//!
//! The document's version now is answered once, and each card carries
//! the version it was made on: a page compares the two to say which
//! cards have gone stale. A card holds no "stale" field of its own,
//! because staleness is a fact about the file now and a card is a fact
//! about the history.

use documents::{Slice, Span};
use kernel::{Address, B3Hash, RunId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalsAnswer {
    pub doc: Address,
    /// The document's version now; `None` when it is missing or cannot
    /// be read.
    pub version: Option<B3Hash>,
    /// The open cards, in the order they were offered.
    pub open: Vec<ProposalCard>,
}

/// One open card: who offered it, on which stretch of which version,
/// and its sentences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalCard {
    pub id: B3Hash,
    pub run: RunId,
    pub baseline: B3Hash,
    pub span: Span,
    pub slices: Vec<Slice>,
}
