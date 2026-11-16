// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A candidate user id read from the GitHub CLI on the machine that
//! serves the city, as a page reads it (`crates/wire/spec/Answer/Github.lean` §8-67).

use serde::{Deserialize, Serialize};

/// What one import asked and what came back. Nothing was saved: the
/// login is a candidate the identity card writes only when the person
/// saves it, through `PutIdentity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GithubLoginAnswer {
    /// The host asked, which the page shows beside the candidate and the
    /// card records as `imported_from` when it is saved.
    pub host: String,
    pub reading: GithubReading,
}

/// What the GitHub CLI said about one host, or the named reason it said
/// nothing. Every reading but `Found` leaves the card as the person left
/// it, and each names the way on that the page offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GithubReading {
    /// The login the host's current authenticated identity has.
    Found { login: String },
    /// No `gh` on the search path of the machine serving the city.
    NoCli,
    /// `gh` is there and this host has nobody signed in.
    NotLoggedIn,
    /// Any other failure: the network, or `gh`'s own error. `exit` is
    /// absent when there was no exit code to read: `gh` would not start,
    /// or was stopped when it outran the wait.
    Failed { exit: Option<i32> },
    /// `gh` outran the wait and this city could not stop it: the process
    /// is still running on the city's machine, and `why` is what went
    /// wrong while stopping it.
    Stuck { why: String },
    /// The host asked for is not a host name, so `gh` was not started.
    NotAHost,
}
