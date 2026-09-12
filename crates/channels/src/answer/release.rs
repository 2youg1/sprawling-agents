// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which release this city is running, and which one npm offers.
//!
//! **A reading of right now, never folded from the Ledger**, on the
//! same grounds as `McpHealth` and `Toolkits` (channels-SPEC section
//! 8-31): which release is newest is a fact about this minute, and a
//! recorded copy would still name last month's release as the newest
//! one. It is also the reason this answer is never sent unasked - the
//! city reaches the registry when somebody presses the button and at no
//! other time.
//!
//! **Two renderings rather than the six numbers behind them.** A page
//! draws `0.0.5-pre.260912` and `2026-09-12`; the arithmetic that
//! decides which of two releases is newer is `kernel::Release`'s and
//! stays there, so the client has nothing to get wrong and no second
//! ordering rule to keep in step.

use kernel::{AxError, ReleaseVerdict};
use serde::{Deserialize, Serialize};

/// One release, in the two spellings a person reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReleaseLine {
    /// `0.0.5-pre.260912` - the version npm carries, which is also what
    /// `bunx sprawling@<version>` takes.
    pub version: String,
    /// `2026-09-12`. A pre-alpha version number says almost nothing
    /// about how old a tree is, and how old it is, is what its reader
    /// most needs to know (CHANGELOG.md, opening note).
    pub released: String,
}

/// Where this city stands against the release channel.
///
/// Exhaustive rather than a pair of optional fields: "you are running a
/// release and here is where it stands", "you built this yourself, so
/// there is nothing to compare" and "the registry could not be read"
/// are three different things for a person to do next, and only the
/// first of them is a version number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReleaseAnswer {
    /// This binary came out of a release, and here is where it stands.
    Stands {
        mine: ReleaseLine,
        newest: ReleaseLine,
        verdict: ReleaseVerdict,
    },
    /// This binary was built from a working tree, so it is none of the
    /// published releases. Reporting it as out of date would be
    /// answering about a binary the person is not running.
    Unreleased { newest: ReleaseLine },
    /// The check could not be made. Carries why, because "could not
    /// check" leaves a person believing they checked.
    Refused { refusal: AxError },
}
