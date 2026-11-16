// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ACP registry's index read as agent entries, and the snapshot of it
//! this build ships (`crates/agent_protocols/Spec.lean` §8-19, D12, D13).
//!
//! One reader for both: the snapshot `cargo xtask acp-catalog` commits is
//! the CDN index with the fields this city does not read taken out, so a
//! full index fetched on refresh and the shipped snapshot are read by the
//! same code.

use kernel::{AxCode, AxError};
use serde_json::Value;

use super::entry::{AgentEntry, AgentId, AgentSource, Launch, Launcher};

/// The shipped snapshot, with the index's `Date` and `ETag` beside it.
const BUNDLED: &str = include_str!("../../catalog/registry.json");

/// The agents of one registry index this machine can start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub entries: Vec<AgentEntry>,
    /// The index's `Date` header.
    pub date: String,
    /// The index's `ETag` header.
    pub etag: String,
}

impl Catalog {
    /// The snapshot this build ships.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` when the committed snapshot does not read, which
    /// `catalog::tests` would have caught before the build shipped.
    pub fn bundled() -> Result<Catalog, AxError> {
        let index: Value = serde_json::from_str(BUNDLED).map_err(|err| unreadable(&err))?;
        let header = |key: &str| text(&index, key).unwrap_or_default().to_owned();
        let (date, etag) = (header("date"), header("etag"));
        Catalog::of(&index, date, etag)
    }

    /// One registry index, as the CDN serves it.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` for text that is not JSON, an index with no
    /// `agents` list, and an agent [`registry_entry`] refuses.
    pub fn read(index: &str, date: String, etag: String) -> Result<Catalog, AxError> {
        let index: Value = serde_json::from_str(index).map_err(|err| unreadable(&err))?;
        Catalog::of(&index, date, etag)
    }

    fn of(index: &Value, date: String, etag: String) -> Result<Catalog, AxError> {
        let agents = index
            .get("agents")
            .and_then(Value::as_array)
            .ok_or_else(|| refused("the index has no `agents` list"))?;
        let mut entries = Vec::new();
        for agent in agents {
            entries.extend(registry_entry(agent, AgentSource::Registry)?);
        }
        Ok(Catalog {
            entries,
            date,
            etag,
        })
    }

    /// The entry the registry lists under `registry_id`.
    #[must_use]
    pub fn find(&self, registry_id: &str) -> Option<&AgentEntry> {
        self.entries
            .iter()
            .find(|entry| entry.id.as_str() == registry_id)
    }
}

/// One registry `agent.json` read as an entry, or `None` when it offers
/// nothing this platform can start.
///
/// `npx` and `uvx` come first, in that order: they are pinned by the
/// package string. A `binary` distribution is not downloaded in this
/// release; its command for this platform is looked for on the search
/// path under its file name, unpinned.
///
/// # Errors
/// `E_WIRE_MISMATCH` for an agent without an `id`, a `name` or a
/// `distribution`, and an id [`AgentId::parse`] refuses.
pub fn registry_entry(agent: &Value, source: AgentSource) -> Result<Option<AgentEntry>, AxError> {
    let field =
        |key: &str| text(agent, key).ok_or_else(|| refused(&format!("an agent has no `{key}`")));
    let id = AgentId::parse(field("id")?)?;
    let name = field("name")?.to_owned();
    let distribution = agent
        .get("distribution")
        .ok_or_else(|| refused(&format!("{}: no `distribution`", id.as_str())))?;
    let Some(launch) = launch_of(distribution) else {
        return Ok(None);
    };
    Ok(Some(AgentEntry {
        id,
        name,
        source,
        launch,
        version: text(agent, "version").map(str::to_owned),
        licence: text(agent, "license").map(str::to_owned),
    }))
}

fn launch_of(distribution: &Value) -> Option<Launch> {
    let packaged = [(Launcher::Npx, "npx"), (Launcher::Uvx, "uvx")]
        .into_iter()
        .find_map(|(launcher, key)| {
            let target = distribution.get(key)?;
            let package = text(target, "package")?.to_owned();
            let lead = match launcher {
                Launcher::Npx => vec!["-y".to_owned(), package],
                Launcher::Uvx => vec![package],
            };
            Some(Launch {
                program: launcher.program().to_owned(),
                args: lead
                    .into_iter()
                    .chain(strings(target.get("args")))
                    .collect(),
                env: pairs(target.get("env")),
            })
        });
    packaged.or_else(|| {
        let target = distribution.get("binary")?.get(platform())?;
        let program = std::path::Path::new(text(target, "cmd")?)
            .file_name()?
            .to_str()?
            .to_owned();
        Some(Launch {
            program,
            args: strings(target.get("args")),
            env: pairs(target.get("env")),
        })
    })
}

/// This build's platform as the registry spells it (`FORMAT.md`).
fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("macos", _) => "darwin-x86_64",
        ("windows", "aarch64") => "windows-aarch64",
        ("windows", _) => "windows-x86_64",
        (_, "aarch64") => "linux-aarch64",
        (_, _) => "linux-x86_64",
    }
}

fn strings(list: Option<&Value>) -> Vec<String> {
    list.and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// A JSON object of strings as name/value pairs, in the order `serde_json`
/// keeps them (sorted), so one entry always yields one digest.
pub(super) fn pairs(table: Option<&Value>) -> Vec<(String, String)> {
    table
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(name, value)| Some((name.clone(), value.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

fn text<'v>(value: &'v Value, key: &str) -> Option<&'v str> {
    value.get(key).and_then(Value::as_str)
}

fn unreadable(err: &serde_json::Error) -> AxError {
    refused(&format!("not JSON: {err}"))
}

fn refused(what: &str) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "read the ACP registry index",
        what.to_owned(),
    )
    .with_recovery("refresh the catalog, or paste the agent's command instead")
}

#[cfg(test)]
mod tests;
