// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{Address, AxError, B3Hash};
use serde::Deserialize;

use super::ladder::{self, Layer};
use super::mcp::{Carried, vaulted};
use super::refuse::refuse;
use super::write::{Change, change, change_at};

pub(crate) const AGENT_KEY: &str = "[[agent]]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    pub id: String,
    pub name: Option<String>,

    pub command: String,
    pub args: Vec<String>,

    pub env: Vec<(String, String)>,
    pub source: AgentRowSource,
    pub version: Option<String>,

    pub launch_digest: Option<B3Hash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRowSource {
    Registry,
    Detected,
    Pasted,
}

pub fn agent_rows(city_root: &Path) -> Result<Vec<AgentRow>, AxError> {
    let file = CityLayout::new(city_root).city_config();
    Ok(ladder::stated(&file, Layer::City)?
        .agents()
        .map(<[AgentRow]>::to_vec)
        .unwrap_or_default())
}

pub fn write_agent(city_root: &Path, row: &AgentRow) -> Result<(), AxError> {
    vaulted(&row.id, Carried::Env, &row.env)?;
    change_at(
        &CityLayout::new(city_root).city_config(),
        Change::Agent(row),
    )
}

pub fn seat_agent(city_root: &Path, room: &Address, id: &str) -> Result<(), AxError> {
    change(city_root, room, Layer::Resident, Change::Seat(id))
}

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
