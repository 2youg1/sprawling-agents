// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The catalog selection one session froze at its own address. It is a
//! session record beside CONFIG.toml, never a third hand-edited layer
//! (`crates/city/spec/ConfigLayers.lean` D114).

use std::collections::BTreeMap;
use std::path::Path;

use kernel::config::CatalogExposure;
use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash, FrozenConfig, McpReference};
use serde::{Deserialize, Serialize};

use crate::document;

/// Everything about a building's offer that one session keeps fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCatalog {
    pub reading_room: Vec<String>,
    #[serde(default)]
    pub skill_pins: BTreeMap<String, B3Hash>,
    #[serde(default)]
    pub mcp_references: Vec<McpReference>,
    #[serde(default)]
    pub catalog: CatalogExposure,
}

impl SessionCatalog {
    /// The selection a new session takes from the building now.
    #[must_use]
    pub fn current(rules: &crate::BuildingRules, config: &FrozenConfig) -> SessionCatalog {
        SessionCatalog {
            reading_room: rules.reading_room().to_vec(),
            skill_pins: config.skill_pins.clone(),
            mcp_references: config.mcp_references.clone(),
            catalog: config.catalog.clone(),
        }
    }

    /// The canonical identity compared when a page counts older sessions.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` if this value cannot be rendered as TOML.
    pub fn digest(&self) -> Result<B3Hash, AxError> {
        toml::to_string(self)
            .map(|text| B3Hash::digest(text.as_bytes()))
            .map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "compare a session catalog",
                    err.to_string(),
                )
                .with_recovery("report this: the session catalog must remain serialisable")
            })
    }

    fn apply(&self, config: &mut FrozenConfig) {
        config.skill_pins.clone_from(&self.skill_pins);
        config.mcp_references.clone_from(&self.mcp_references);
        config.catalog.clone_from(&self.catalog);
    }
}

/// Reads the session's frozen selection, or freezes the building's current
/// one on its first run. The file is read again under the document lock so
/// two first runs cannot both replace it.
pub fn catalog_for_session(
    city_root: &Path,
    addr: &Address,
    rules: &crate::BuildingRules,
    config: &mut FrozenConfig,
) -> Result<SessionCatalog, AxError> {
    let path = CityLayout::new(city_root).session_catalog(addr);
    document::edit(&path, |held| match std::fs::read_to_string(&path) {
        Ok(text) => {
            let frozen: SessionCatalog = toml::from_str(&text).map_err(|err| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "read a session catalog",
                        format!("{}: {err}", path.display()),
                    )
                    .with_recovery(
                        "repair CATALOG.toml, or start a new session to freeze the building's current selection",
                    )
                })?;
            frozen.apply(config);
            Ok(frozen)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let current = SessionCatalog::current(rules, config);
            let text = toml::to_string_pretty(&current).map_err(|err| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "write a session catalog",
                    format!("{}: {err}", path.display()),
                )
                .with_recovery("report this: the current selection must remain serialisable")
            })?;
            held.replace(text.as_bytes())?;
            Ok(current)
        }
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a session catalog",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("make CATALOG.toml readable, then dispatch again")),
    })
}

/// Fills a legacy connector reference in this session only. A missing
/// selection is left absent: migration never admits another server.
pub fn migrate_session_mcp_reference(
    city_root: &Path,
    addr: &Address,
    reference: &McpReference,
) -> Result<(), AxError> {
    migrate(city_root, addr, |frozen| {
        if let Some(selected) = frozen
            .mcp_references
            .iter_mut()
            .find(|selected| selected.label == reference.label)
        {
            if selected.definition.is_none() {
                selected.definition = reference.definition;
            }
            if selected.tools.is_none() {
                selected.tools.clone_from(&reference.tools);
            }
        }
    })
}

/// Fills one legacy skill pin already named by this session's Reading Room.
pub fn migrate_session_skill_pin(
    city_root: &Path,
    addr: &Address,
    name: &str,
    digest: B3Hash,
) -> Result<(), AxError> {
    migrate(city_root, addr, |frozen| {
        if frozen.reading_room.iter().any(|named| named == name) {
            frozen.skill_pins.entry(name.to_owned()).or_insert(digest);
        }
    })
}

/// Applies a first-run pin fill under the session document lock. A session
/// removed by `/new` concurrently has no record left to migrate.
fn migrate(
    city_root: &Path,
    addr: &Address,
    change: impl FnOnce(&mut SessionCatalog),
) -> Result<(), AxError> {
    let path = CityLayout::new(city_root).session_catalog(addr);
    document::edit(&path, |held| match std::fs::read_to_string(&path) {
        Ok(text) => {
            let mut frozen: SessionCatalog = toml::from_str(&text).map_err(|err| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "migrate a session catalog",
                    format!("{}: {err}", path.display()),
                )
                .with_recovery("repair CATALOG.toml, or start a new session")
            })?;
            change(&mut frozen);
            let rendered = toml::to_string_pretty(&frozen).map_err(|err| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "migrate a session catalog",
                    format!("{}: {err}", path.display()),
                )
                .with_recovery("report this: the session catalog must remain serialisable")
            })?;
            held.replace(rendered.as_bytes())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "migrate a session catalog",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("make CATALOG.toml readable, then dispatch again")),
    })
}

/// Forgets the frozen selection when `/new` starts another session.
pub fn forget(city_root: &Path, addr: &Address) -> Result<(), AxError> {
    let path = CityLayout::new(city_root).session_catalog(addr);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "forget a session catalog",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("make CATALOG.toml removable, then start the new session again")),
    }
}

/// How many existing sessions froze a selection other than `current`.
/// The building's own root session counts beside its named rooms.
pub fn previous_sessions(
    city_root: &Path,
    building: &Address,
    current: &SessionCatalog,
) -> Result<u32, AxError> {
    let current = current.digest()?;
    let mut count = 0_u32;
    for addr in std::iter::once(building.clone()).chain(crate::rooms(city_root, building)?) {
        let path = CityLayout::new(city_root).session_catalog(&addr);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(AxError::failure(
                    AxCode::StorageFatal,
                    "count sessions on an older catalog",
                    format!("{}: {err}", path.display()),
                )
                .with_recovery("make the session catalog readable, then ask again"));
            }
        };
        let frozen: SessionCatalog = toml::from_str(&text).map_err(|err| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "count sessions on an older catalog",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("repair the session catalog, or start that session anew")
        })?;
        if frozen.digest()? != current {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}
