// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The proposal cards still open on one document, and every document of
//! the city that has one (`crates/wire/spec/Answer/Proposals.lean` §8-73).
//!
//! The document's version now is answered once, and each card carries
//! the version it was made on: a page compares the two to say which
//! cards have gone stale. A card holds no "stale" field of its own,
//! because staleness is a fact about the file now and a card is a fact
//! about the history.

use documents::{Slice, Span};
use kernel::{Address, B3Hash, RunId, TimeMs};
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

/// Every card still open in the city, newest offer first: the list a
/// person deciding reads before they open any one document.
///
/// Names and moments only. A card's sentences and the document's
/// version are [`ProposalsAnswer`]'s, asked per document, so the diff
/// of a card nobody looks at is never laid out to be counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OpenProposalsAnswer {
    pub open: Vec<OfferedCard>,
}

/// One open card, by the document it is on and the moment it was
/// offered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OfferedCard {
    pub doc: Address,
    pub id: B3Hash,
    /// When the `proposal_offered` line was written; absent only for a
    /// card the answering fold holds without having seen its line.
    pub at: Option<TimeMs>,
}
