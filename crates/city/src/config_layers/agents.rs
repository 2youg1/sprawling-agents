// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[[agent]]` table: the ACP agents the person added to this city,
//! one row each, in the city's own `CONFIG.toml` only.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4f. Which agent an
//! id names and whether a row's command is the one consented to are
//! `agent_protocols`'s to answer; this module owns how a row is written
//! and what is refused where it is written.

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxError, B3Hash};
use serde::Deserialize;

use super::ladder::{self, Layer};
use super::mcp::{Carried, vaulted};
use super::refuse::refuse;

/// This table's header, spelled once for every refusal that names it.
pub(crate) const AGENT_KEY: &str = "[[agent]]";

/// One agent the person added, as its row states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    /// What `[resident] harness` names it by.
    pub id: String,
    pub name: Option<String>,
    /// The program, an absolute path once the consent card resolved it.
    pub command: String,
    pub args: Vec<String>,
    /// Name before value, in the order a `BTreeMap` reads them. A value
    /// may be a `secret:realm/name` reference; a credential written out is
    /// refused.
    pub env: Vec<(String, String)>,
    pub source: AgentRowSource,
    pub version: Option<String>,
    /// The digest of the launch the person consented to; absent on a row
    /// the person wrote by hand, whose file is the consent.
    pub launch_digest: Option<B3Hash>,
}

/// Where a row's agent came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRowSource {
    Registry,
    Detected,
    Pasted,
}

/// The agents the city's own `CONFIG.toml` adds; none when it adds none.
///
/// # Errors
/// Refuses an unreadable file and one that does not parse, exactly as
/// [`super::load`] does.
pub fn agent_rows(city_root: &Path) -> Result<Vec<AgentRow>, AxError> {
    let file = CityLayout::new(city_root).city_config();
    Ok(ladder::stated(&file, Layer::City)?
        .agents()
        .map(<[AgentRow]>::to_vec)
        .unwrap_or_default())
}

/// Reads one layer's `[[agent]]` rows.
///
/// # Errors
/// Refuses an empty id or command, two rows under one id, and an `env`
/// value that is a credential rather than a vault reference.
pub(super) fn rows(entries: Vec<AgentSection>) -> Result<Vec<AgentRow>, AxError> {
    let mut rows: Vec<AgentRow> = Vec::new();
    for entry in entries {
        if entry.id.trim().is_empty() || entry.command.trim().is_empty() {
            return Err(refuse(format!(
                "`{AGENT_KEY}` `{}`: an agent needs an id and a command",
                entry.id
            )));
        }
        if rows.iter().any(|held| held.id == entry.id) {
            return Err(refuse(format!(
                "`{AGENT_KEY}` `{}`: two agents are named the same",
                entry.id
            )));
        }
        let env: Vec<(String, String)> = entry.env.into_iter().collect();
        vaulted(&entry.id, Carried::Env, &env)?;
        rows.push(AgentRow {
            id: entry.id,
            name: entry.name,
            command: entry.command,
            args: entry.args,
            env,
            source: entry.source,
            version: entry.version,
            launch_digest: entry.launch_digest,
        });
    }
    Ok(rows)
}

/// One `[[agent]]` row, read as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentSection {
    id: String,
    #[serde(default)]
    name: Option<String>,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    source: AgentRowSource,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    launch_digest: Option<B3Hash>,
}
