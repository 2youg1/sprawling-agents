// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[catalog]` table: how each admitted tool reaches a building's
//! runs, stated per tool or per namespace, read from the file, judged and
//! written back whole.
//!
//! The order a run reads the city's and the building's tables in is
//! `kernel::config::CatalogExposure::stated`'s; which tools no table can
//! move and what an unstated tool falls back to is `runtime::catalog`'s.
//! This module answers what one file says.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4g.

use std::collections::BTreeMap;
use std::path::Path;

use kernel::config::{Exposure, ExposureTable};
use kernel::{Address, AxError, ServerLabel, ToolName};
use serde::{Deserialize, Serialize};

use super::ConfigLayer;
use super::ladder::Layer;
use super::refuse::refuse_with;
use super::write::{Change, change};

/// The table's header, as a refusal names it.
pub(crate) const CATALOG_KEY: &str = "[catalog]";

/// The `[catalog]` table as a file spells it: `[catalog.tools]` and
/// `[catalog.namespaces]`, each a name to `"core"`, `"dormant"` or
/// `"off"`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogSection {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    tools: BTreeMap<String, Exposure>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    namespaces: BTreeMap<String, Exposure>,
}

/// Reads one layer's `[catalog]` table into the value it states.
///
/// # Errors
/// `E_CONFIG_INVALID` for a key that is not a tool name, or not a
/// namespace name; [`check`] says which.
pub(super) fn stated(section: CatalogSection) -> Result<ExposureTable, AxError> {
    let table = ExposureTable {
        tools: section.tools,
        namespaces: section.namespaces,
    };
    check(&table)?;
    Ok(table)
}

/// `table` in the file's own syntax, for the write face; `None` for a
/// table that states nothing, which the writer takes out of the file.
///
/// # Errors
/// `E_CONFIG_INVALID` when the value cannot be spelled as TOML.
pub(super) fn spelled(table: &ExposureTable, file: &Path) -> Result<Option<toml::Value>, AxError> {
    if table.is_empty() {
        return Ok(None);
    }
    let section = CatalogSection {
        tools: table.tools.clone(),
        namespaces: table.namespaces.clone(),
    };
    toml::Value::try_from(section)
        .map(Some)
        .map_err(|err| refuse_with(format!("{}: {err}", file.display()), CHANGE_THE_KEY))
}

/// Writes a building's whole `[catalog]` table, replacing what its
/// layer stated before; an empty table takes the key out, so the
/// building inherits the city's again. Judged first, by the reader's own
/// check. The city's own table is written through
/// [`super::write_city_setting`].
///
/// # Errors
/// `E_CONFIG_INVALID`, before any byte is written, for every key the
/// reader would refuse; refuses an address with no building; propagates a
/// layer that exists and cannot be read or parsed, and a directory that
/// cannot be written.
pub fn write_catalog(
    city_root: &Path,
    building: &Address,
    table: &ExposureTable,
) -> Result<(), AxError> {
    check(table)?;
    change(city_root, building, Layer::Building, Change::Catalog(table))
}

/// What a person does when a key is not a name this table takes.
const CHANGE_THE_KEY: &str = "name a tool as its catalog lists it, or a namespace in ascii \
                              lowercase and digits, or take that key out";

/// Judges a whole table, for the reader and the writer alike: every tool
/// key spells a tool name and every namespace key a namespace, so a
/// statement that could never match anything is refused where it is
/// written rather than silently ignored. A name no tool has today is
/// accepted: a server's tools come and go with the server.
///
/// The writers call it themselves; a frame that writes another refusable
/// fact beside the table calls it first, so neither lands when the table
/// is refused (`crates/wire/spec/Command/Step.lean` §8-61).
///
/// # Errors
/// `E_CONFIG_INVALID` naming the first key that is not a tool name or a
/// namespace.
pub fn check(table: &ExposureTable) -> Result<(), AxError> {
    for tool in table.tools.keys() {
        ToolName::parse(tool).map_err(|_| {
            refuse_with(
                format!("{CATALOG_KEY} tools: `{tool}` is not a tool name"),
                CHANGE_THE_KEY,
            )
        })?;
    }
    for namespace in table.namespaces.keys() {
        ServerLabel::parse(namespace).map_err(|_| {
            refuse_with(
                format!("{CATALOG_KEY} namespaces: `{namespace}` is not a namespace"),
                CHANGE_THE_KEY,
            )
        })?;
    }
    Ok(())
}

/// `ConfigLayer`'s reading of its own `[catalog]`.
impl ConfigLayer {
    /// What this layer says about how its tools reach a run, as written.
    #[must_use]
    pub fn catalog(&self) -> Option<&ExposureTable> {
        self.catalog.as_ref()
    }
}
