// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The harness page: each official harness, the command that starts it
//! as an ACP agent, and whether this machine can run that command
//! (`crates/wire/spec/Answer/Harnesses.lean` §8-52).
//!
//! Nothing here is about a credential: the person signs in inside the
//! harness, and `docs` is where its own vendor says how.

use serde::{Deserialize, Serialize};

/// Every official harness, in the roster's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessesAnswer {
    pub harnesses: Vec<HarnessLine>,
}

/// One harness as the page draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessLine {
    /// The roster's word for it: `claude_code`, `codex`, `grok_build`,
    /// `kimi_code` or `pi`.
    pub name: String,
    /// The command that starts it as an ACP agent, word by word.
    pub launch: Vec<String>,
    /// Whether this machine can use it (`crates/wire/spec/Answer/Harnesses.lean` D23).
    pub state: HarnessState,
    /// Where its vendor says how a person signs in.
    pub docs: String,
}

/// What a person does next about one harness: install its launcher,
/// install or sign in to the harness, or use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HarnessState {
    /// The launcher program (mostly `npx`) is not on the search path.
    LauncherMissing { program: String },
    /// The launcher is there, and none of the harness's install or
    /// sign-in directories is. `looked` lists every directory checked,
    /// expanded; empty while the harness has no directories registered.
    NotSetUp { looked: Vec<String> },
    /// The directory found.
    Ready { at: String },
}
