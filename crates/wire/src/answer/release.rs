// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which release this city is running, and which one npm offers.
//!
//! **A reading of right now, never folded from the Ledger**, on the
//! same grounds as `McpHealth` and `Toolkits` (`crates/wire/spec/Answer/McpHealth.lean`
//! §8-34): which release is newest is a fact about this minute, and a
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
    /// `verdict` judges `mine` against npm's line, which is what
    /// `bunx sprawling` resolves.
    Stands {
        mine: ReleaseLine,
        registries: Vec<RegistryNewest>,
        verdict: ReleaseVerdict,
        update: UpdateHint,
    },
    /// This binary was built from a working tree, so it is none of the
    /// published releases. Reporting it as out of date would be
    /// answering about a binary the person is not running.
    Unreleased {
        registries: Vec<RegistryNewest>,
        update: UpdateHint,
    },
    /// The check could not be made. Carries why, because "could not
    /// check" leaves a person believing they checked.
    Refused { refusal: AxError },
}

/// What one registry said about the newest release
/// (`crates/wire/spec/Answer/Release.lean` D24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RegistryNewest {
    pub registry: Registry,
    pub reading: RegistryReading,
}

/// A registry this project publishes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Registry {
    Npm,
    CratesIo,
}

/// What one registry answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RegistryReading {
    Read {
        newest: ReleaseLine,
    },
    Refused {
        refusal: AxError,
    },
    /// Not asked: no rule yet compares this registry's version with
    /// this binary's, so an answer would be a guess.
    Unasked,
}

/// How this binary was installed, which decides the command that
/// updates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum InstallChannel {
    Npm,
    Cargo,
    Archive,
    Source,
}

/// The command a person runs to update, printed and never run: the
/// channel that installed the binary owns updating it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpdateHint {
    pub channel: InstallChannel,
    /// `None` for a binary built from source, which nothing updates.
    pub command: Option<String>,
}
