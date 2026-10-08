// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ACP page: the agent catalog, one pasted spec read as an agent
//! entry, and the agents this city added
//! (`crates/wire/spec/Answer/Agents.lean` §8-90).

use kernel::{Address, B3Hash};
use serde::{Deserialize, Serialize};

/// Everything the ACP page draws from one question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentCatalogAnswer {
    /// Agents found on this machine without running anything; each one is
    /// a consent card of its own.
    pub detected: Vec<AgentOffer>,
    /// The catalog snapshot shipped with this build, the built-in entries
    /// among them.
    pub catalog: Vec<AgentOffer>,
    /// The agents this city added.
    pub added: Vec<AgentLine>,
    pub snapshot: CatalogSnapshot,
}

/// When the shipped catalog was read from the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogSnapshot {
    /// The registry's `Date` header.
    pub date: String,
    /// The registry's `ETag` header.
    pub etag: String,
}

/// One agent the person may consent to, as the consent card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentOffer {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    /// The exact command line, shown as it is.
    pub launch_preview: String,
    pub version: Option<String>,
    pub pinned: PinState,
    pub licence: Option<String>,
    /// The names of the environment variables the entry sets; never their
    /// values.
    pub env_names: Vec<String>,
    pub login: Vec<LoginKind>,
    /// The digest of this launch spec; `AddAgent` carries it back as the
    /// consent.
    pub spec_digest: B3Hash,
}

/// Where an agent entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AgentSource {
    /// The ACP registry's catalog, as the build shipped it.
    Registry,
    /// Found on this machine.
    Detected,
    /// Pasted by the person.
    Pasted,
}

/// How exactly the launch spec names the agent's version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PinState {
    /// One exact version (`pkg@1.2.3`).
    Exact,
    /// Whatever the launcher resolves (`@latest`, or no version).
    Floating,
    /// A program found on this machine; the city ran nothing, so it does
    /// not know which version.
    Unknown,
}

/// The two auth method types of ACP's stable schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginKind {
    /// The agent signs in through `authenticate`.
    Agent,
    /// The agent signs in on a terminal of its own.
    Terminal,
}

/// One agent this city added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentLine {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    pub version: Option<String>,
    pub pinned: PinState,
    pub login_state: LoginState,
    /// The methods the agent declared in `initialize`, without the one
    /// login this city never starts on a person's behalf.
    pub auth_methods: Vec<AuthMethod>,
    /// The rooms whose `[resident] harness` names this agent.
    pub seated_in: Vec<Address>,
}

/// Where an added agent stands on signing in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginState {
    /// The city has not opened a session with it yet.
    Unasked,
    /// It took a session without asking for a login.
    Ready,
    /// It answered `-32000`: the page offers its login.
    Required,
}

/// One auth method an agent declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuthMethod {
    pub id: String,
    pub name: String,
    pub kind: LoginKind,
}
