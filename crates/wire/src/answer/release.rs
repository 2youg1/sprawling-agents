// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which version this city runs, and what each installation registry offers.
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
    /// The registry's version spelling: dated npm semver or a bare crates.io version.
    pub version: String,
    /// The release date when the registry's spelling carries one; empty when unknown.
    pub released: String,
}

/// Where this city stands against the release channel.
///
/// A comparison, an unreleased build, an origin requiring confirmation, or a refusal.
/// The selected registry supplies both the displayed version and the comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReleaseAnswer {
    /// This binary can be compared with its installation registry.
    /// `newest` and `verdict` come from the registry this channel updates from.
    Stands {
        mine: ReleaseLine,
        /// The exact registry version used for `verdict`, selected by the server.
        newest: ReleaseLine,
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
    /// A published binary whose installation origin needs the person to confirm it.
    Unconfirmed {
        mine: ReleaseLine,
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
    Github,
}

/// What one registry answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RegistryReading {
    Read { newest: ReleaseLine },
    Refused { refusal: AxError },
}

/// How this binary was installed, which decides the command that
/// updates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum InstallChannel {
    Npm,
    Bun,
    Binstall,
    Unknown,
    Package,
    CargoOrBinstall,
    Cargo,
    Archive,
    Homebrew,
    Aur,
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
    /// Exact commands requiring explicit choice when the path cannot identify an installer.
    pub alternatives: Vec<String>,
}
