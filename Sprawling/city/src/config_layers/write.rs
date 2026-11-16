// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::config::SecondThreshold;
use kernel::{Address, AxError, B3Hash, McpDefinition, McpReference, McpServer, SandboxLimits};

use super::{ConfigLayer, Layer, path};
use crate::document;

mod change;
pub(super) use change::{Change, change, change_at};
use change::{read_document, reference_rows, refuse_file, rows};

pub fn migrate_mcp(city_root: &Path, building: &Address) -> Result<bool, AxError> {
    let file = path(city_root, building, Layer::Building)?;
    if !file.exists() {
        return Ok(false);
    }
    let city = super::ladder::stated(
        &kernel::layout::CityLayout::new(city_root).city_config(),
        Layer::City,
    )?;
    let mut migrated = false;
    document::edit(&file, |held| {
        let mut document = read_document(&file)?;
        let original = toml::to_string_pretty(&document)
            .map_err(|err| refuse_file(&file, &err.to_string()))?;
        let layer =
            ConfigLayer::parse(&original).map_err(|err| refuse_file(&file, err.subject()))?;
        let mut references = layer
            .mcp()
            .and_then(|mcp| mcp.references.clone())
            .unwrap_or_default();
        let mut changed = false;
        let mut definitions = Vec::new();
        for definition in layer
            .mcp()
            .and_then(|mcp| mcp.definitions.clone())
            .unwrap_or_default()
        {
            let farther = city.mcp().and_then(|mcp| {
                mcp.definitions
                    .as_ref()?
                    .iter()
                    .find(|candidate| candidate.server.label == definition.server.label)
            });
            match farther {
                Some(candidate) if candidate.server == definition.server => {
                    references.push(McpReference::legacy(definition.server.label));
                    changed = true;
                }
                Some(_) if !definition.conflict => {
                    definitions.push(McpDefinition {
                        conflict: true,
                        ..definition
                    });
                    changed = true;
                }
                Some(_) | None => definitions.push(definition),
            }
        }
        if !changed {
            return Ok(());
        }
        migrated = true;
        references.sort_by(|left, right| left.label.as_str().cmp(right.label.as_str()));
        references.dedup_by(|left, right| left.label == right.label);
        document.insert(
            super::mcp::MCP_REFERENCE_FIELD.to_owned(),
            toml::Value::Array(reference_rows(&references)),
        );
        if definitions.is_empty() {
            document.remove("mcp");
        } else {
            let servers = definitions
                .iter()
                .map(|definition| definition.server.clone())
                .collect::<Vec<_>>();
            let mut rows = rows(&servers);
            for (row, definition) in rows.iter_mut().zip(definitions.iter()) {
                if definition.conflict
                    && let toml::Value::Table(row) = row
                {
                    row.insert("conflict".to_owned(), toml::Value::Boolean(true));
                }
            }
            document.insert("mcp".to_owned(), toml::Value::Array(rows));
        }
        let rendered = toml::to_string_pretty(&document)
            .map_err(|err| refuse_file(&file, &err.to_string()))?;
        ConfigLayer::parse(&rendered).map_err(|err| refuse_file(&file, err.subject()))?;
        held.replace(rendered.as_bytes())
    })?;
    Ok(migrated)
}

pub fn write_sandbox(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    limits: &SandboxLimits,
) -> Result<(), AxError> {
    change(city_root, addr, layer, Change::Sandbox(limits))
}

pub fn write_second_threshold(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    threshold: SecondThreshold,
) -> Result<(), AxError> {
    change(city_root, addr, layer, Change::SecondThreshold(threshold))
}

pub fn write_mcp(
    city_root: &Path,
    addr: &Address,
    layer: Layer,
    servers: &[McpServer],
) -> Result<(), AxError> {
    super::mcp::validate(servers)?;
    change(city_root, addr, layer, Change::Mcp(servers))
}

pub fn write_mcp_references(
    city_root: &Path,
    building: &Address,
    references: &[McpReference],
) -> Result<(), AxError> {
    super::check_mcp_references(city_root, building, references)?;
    change(
        city_root,
        building,
        Layer::Building,
        Change::McpReferences(references),
    )
}

pub fn write_skill_pins(
    city_root: &Path,
    building: &Address,
    pins: &std::collections::BTreeMap<String, B3Hash>,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::SkillPins(pins),
    )
}

pub fn migrate_mcp_reference(
    city_root: &Path,
    building: &Address,
    reference: &McpReference,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::MigrateMcpReference(reference),
    )
}

pub fn migrate_skill_pin(
    city_root: &Path,
    building: &Address,
    name: &str,
    digest: B3Hash,
) -> Result<(), AxError> {
    change(
        city_root,
        building,
        Layer::Building,
        Change::MigrateSkillPin { name, digest },
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
