// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's `[[agent]]` rows and the shipped catalog, read as the one
//! roster a word is seated from (`crates/agent_protocols/Spec.lean` §8-19,
//! `crates/city/spec/ConfigLayers.lean` §8-4f).
//!
//! It lives here because `city` sees only `kernel` and `agent_protocols`
//! sees no configuration: this crate is the first that sees both, and the
//! dispatch path and the ACP page read the same roster through it.

use std::path::Path;

use agent_protocols::{AgentEntry, AgentId, AgentSource, Catalog, Launch, Roster};
use kernel::{AxError, B3Hash};

/// The roster of the city at `city_root`.
///
/// # Errors
/// Refuses an unreadable or unparsable city `CONFIG.toml`, a row whose id
/// is not an agent id, and a shipped snapshot that does not read.
pub(crate) fn roster(city_root: &Path) -> Result<Roster, AxError> {
    let rows = city::agent_rows(city_root)?
        .into_iter()
        .map(entry_of)
        .collect::<Result<Vec<_>, AxError>>()?;
    Ok(Roster::new(rows, Catalog::bundled()?))
}

/// One row as the entry it adds, with the digest its consent recorded.
fn entry_of(row: city::AgentRow) -> Result<(AgentEntry, Option<B3Hash>), AxError> {
    let id = AgentId::parse(&row.id)?;
    let entry = AgentEntry {
        name: row.name.unwrap_or_else(|| id.as_str().to_owned()),
        id,
        source: match row.source {
            city::AgentRowSource::Registry => AgentSource::Registry,
            city::AgentRowSource::Detected => AgentSource::Detected,
            city::AgentRowSource::Pasted => AgentSource::Pasted,
        },
        launch: Launch {
            program: row.command,
            args: row.args,
            env: row.env,
        },
        version: row.version,
        licence: None,
    };
    Ok((entry, row.launch_digest))
}
