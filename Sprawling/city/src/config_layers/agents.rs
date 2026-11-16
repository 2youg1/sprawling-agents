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
use kernel::{Address, AxError, B3Hash};
use serde::Deserialize;

use super::ladder::{self, Layer};
use super::mcp::{Carried, vaulted};
use super::refuse::refuse;
use super::write::{Change, change, change_at};

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

/// Writes `row` into the city's own `CONFIG.toml`, in place of the row
/// with its id or after the last one, every other row as it was.
///
/// # Errors
/// Refuses an `env` value that is a credential rather than a vault
/// reference before a byte is written, and a file the reader would then
/// refuse; propagates a file that cannot be read, parsed or written.
pub fn write_agent(city_root: &Path, row: &AgentRow) -> Result<(), AxError> {
    vaulted(&row.id, Carried::Env, &row.env)?;
    change_at(
        &CityLayout::new(city_root).city_config(),
        Change::Agent(row),
    )
}

/// Names the agent `id` as the resident of `room`, from its next
/// session: the room's own `[resident] harness`, with the session record
/// that layer held taken out, because one layer names a model or a
/// harness and never both.
///
/// # Errors
/// Propagates an address with no room layer, and a file that cannot be
/// read, parsed or written.
pub fn seat_agent(city_root: &Path, room: &Address, id: &str) -> Result<(), AxError> {
    change(city_root, room, Layer::Resident, Change::Seat(id))
}

/// `row` as the table one `[[agent]]` entry is written as, in the
/// reader's own spelling, so the file reads back through [`rows`].
pub(super) fn spelled(row: &AgentRow) -> toml::Table {
    let text = |value: &str| toml::Value::String(value.to_owned());
    let mut table = toml::Table::new();
    table.insert("id".to_owned(), text(&row.id));
    if let Some(name) = &row.name {
        table.insert("name".to_owned(), text(name));
    }
    table.insert("command".to_owned(), text(&row.command));
    table.insert(
        "args".to_owned(),
        toml::Value::Array(row.args.iter().map(|arg| text(arg)).collect()),
    );
    table.insert(
        "env".to_owned(),
        toml::Value::Table(
            row.env
                .iter()
                .map(|(name, value)| (name.clone(), text(value)))
                .collect(),
        ),
    );
    let source = match row.source {
        AgentRowSource::Registry => "registry",
        AgentRowSource::Detected => "detected",
        AgentRowSource::Pasted => "pasted",
    };
    table.insert("source".to_owned(), text(source));
    if let Some(version) = &row.version {
        table.insert("version".to_owned(), text(version));
    }
    if let Some(digest) = &row.launch_digest {
        table.insert("launch_digest".to_owned(), text(&digest.to_string()));
    }
    table
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
