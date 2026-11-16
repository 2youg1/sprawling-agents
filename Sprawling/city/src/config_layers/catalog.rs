// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use kernel::config::{Exposure, ExposureTable};
use kernel::{Address, AxError, ServerLabel, ToolName};
use serde::{Deserialize, Serialize};

use super::ConfigLayer;
use super::ladder::Layer;
use super::refuse::refuse_with;
use super::write::{Change, change};

pub(crate) const CATALOG_KEY: &str = "[catalog]";

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogSection {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    tools: BTreeMap<String, Exposure>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    namespaces: BTreeMap<String, Exposure>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    skills: BTreeMap<String, Exposure>,
}

pub(super) fn stated(section: CatalogSection) -> Result<ExposureTable, AxError> {
    let table = ExposureTable {
        tools: section.tools,
        namespaces: section.namespaces,
        skills: section.skills,
    };
    check(&table)?;
    Ok(table)
}

pub(super) fn spelled(table: &ExposureTable, file: &Path) -> Result<Option<toml::Value>, AxError> {
    if table.is_empty() {
        return Ok(None);
    }
    let section = CatalogSection {
        tools: table.tools.clone(),
        namespaces: table.namespaces.clone(),
        skills: table.skills.clone(),
    };
    toml::Value::try_from(section)
        .map(Some)
        .map_err(|err| refuse_with(format!("{}: {err}", file.display()), CHANGE_THE_KEY))
}

pub fn write_catalog(
    city_root: &Path,
    building: &Address,
    table: &ExposureTable,
) -> Result<(), AxError> {
    check(table)?;
    change(city_root, building, Layer::Building, Change::Catalog(table))
}

const CHANGE_THE_KEY: &str = "name a tool as its catalog lists it, or a namespace in ascii \
                              lowercase and digits, or take that key out";

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
    for skill in table.skills.keys() {
        if skill.is_empty() || skill.starts_with('.') || skill.contains(['/', '\\', ':']) {
            return Err(refuse_with(
                format!("{CATALOG_KEY} skills: `{skill}` is not a shelf name"),
                CHANGE_THE_KEY,
            ));
        }
    }
    Ok(())
}

impl ConfigLayer {

    #[must_use]
    pub fn catalog(&self) -> Option<&ExposureTable> {
        self.catalog.as_ref()
    }
}
