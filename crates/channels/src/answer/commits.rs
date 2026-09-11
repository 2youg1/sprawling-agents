// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city says about a commit it made: one, by oid, or a page of
//! them, newest first.
//!
//! Its own file so that the two questions share one vocabulary in one
//! place: a commit is the same eight facts whether it was looked up or
//! listed, and a second shape for the list would be a second authority
//! on what a commit is.

use kernel::{Address, Effort, GitOid, RunId, Seq, SessionName, TimeMs};
use serde::{Deserialize, Serialize};

/// Which run wrote one commit, in the words the commit's own git
/// trailers use.
///
/// Four of the five trailers appear here; `Sprawling-City` does not,
/// because whoever asked this question already holds the city. `seq` is
/// what the trailers cannot carry: where in the one history the line
/// announcing this commit sits, so a reader can go on from there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitAnswer {
    pub oid: GitOid,
    pub run: RunId,
    /// `Sprawling-Actor`: the address the run worked at.
    pub actor: Address,
    /// `Sprawling-Model`. Empty when the record predates the city
    /// writing the model down: an absent id beats an invented one.
    pub model: String,
    /// `Sprawling-Effort`. Absent leaves the choice to the provider,
    /// which is not the same fact as `Effort::None`.
    pub effort: Option<Effort>,
    /// Where the line announcing this commit sits in the one history.
    pub seq: Seq,
    /// When that line was written: the record's own clock, so a list
    /// of commits reads as a day rather than as a column of numbers.
    pub at: TimeMs,
    /// What a person called this line of work, read off the room the
    /// actor worked in. Absent when the run worked at a building's own
    /// address, which is a run nobody opened a session for.
    pub session: Option<SessionName>,
    /// This run first, then each run it replaced by succession, back to
    /// the first. One entry for a run that replaced nobody.
    pub lineage: Vec<RunId>,
}

/// One page of the commits a city made, newest first.
///
/// `building` and `before` are the question handed back: the wire
/// carries no request id, so a page matches an answer to what it asked
/// by content, the way `ChangesAnswer` carries `base` and `head`. `more`
/// rather than a cursor: the next page begins before the last `seq`
/// here, which the reader already holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitsAnswer {
    pub building: Option<Address>,
    pub before: Option<Seq>,
    pub commits: Vec<CommitAnswer>,
    pub more: bool,
}
