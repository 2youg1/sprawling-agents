// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash};
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};

pub(super) const HOLDING_EXT: &str = "md";

pub(super) fn holding_name(file_name: &str) -> Option<String> {
    let name = file_name.strip_suffix(HOLDING_EXT)?;
    let name = name.strip_suffix('.')?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_owned())
}

pub(super) fn holding_file_name(name: &str) -> String {
    format!("{name}.{HOLDING_EXT}")
}

pub(super) fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OwnShelf {
    Library,
    Building(Address),
}

impl OwnShelf {

    pub(super) fn root(&self, layout: &CityLayout) -> PathBuf {
        match self {
            OwnShelf::Library => layout.library(),
            OwnShelf::Building(building) => layout.building_skills(building),
        }
    }

    pub(super) fn as_shelf(&self) -> Shelf {
        match self {
            OwnShelf::Library => Shelf::Library,
            OwnShelf::Building(building) => Shelf::Building(building.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ShelfKey(String);

impl ShelfKey {

    pub(super) fn of(name: &str) -> ShelfKey {
        ShelfKey(name.trim().to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shelf {

    Library,

    Building(Address),

    External { index: u32, path: String },
}

impl Shelf {
    #[must_use]
    pub fn is_external(&self) -> bool {
        matches!(self, Shelf::External { .. })
    }

    #[must_use]
    pub(super) fn catalog_position(&self) -> (u8, u32) {
        match self {
            Shelf::Library => (0, 0),
            Shelf::Building(_) => (1, 0),
            Shelf::External { index, .. } => (2, *index),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    pub name: String,
    pub section: String,

    pub disclosure: String,

    pub hash: B3Hash,

    pub shelf: Shelf,

    pub carried: Option<String>,
}

pub(super) struct HoldingSource<'a> {
    pub name: String,
    pub section: String,
    pub text: &'a str,
    pub shelf: Shelf,
    pub hash: B3Hash,
}

impl Holding {

    pub(super) fn of(source: HoldingSource<'_>) -> Holding {
        let carried = source.shelf.is_external().then(|| source.text.to_owned());
        Holding {
            name: source.name,
            section: source.section,
            disclosure: disclosure(source.text),
            hash: source.hash,
            carried,
            shelf: source.shelf,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShelfRow {
    pub section: String,
    pub name: String,
    pub digest: B3Hash,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShelfDocument {
    version: u16,
    #[serde(default)]
    skill: Vec<ShelfRow>,
}

const SHELF_VERSION: u16 = 1;

pub(super) fn read_index(root: &Path) -> Result<Vec<ShelfRow>, AxError> {
    let path = root.join(kernel::layout::SHELF_INDEX);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read a skill shelf index",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("make the index readable, or restore it through the migration door")
    })?;
    let document: ShelfDocument = toml::from_str(&text).map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "read a skill shelf index",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("restore the index from the shelf's verified CAS objects")
    })?;
    if document.version != SHELF_VERSION {
        return Err(index_error(&path, "unsupported version"));
    }
    validate_rows(&document.skill).map_err(|why| index_error(&path, &why))?;
    Ok(document.skill)
}

pub(super) fn upsert_index(root: &Path, row: ShelfRow) -> Result<(), AxError> {
    let mut rows = read_index(root)?;
    if let Some(at) = rows.iter().position(|held| held.name == row.name) {
        let existing = rows
            .get(at)
            .ok_or_else(|| index_error(&root.join(kernel::layout::SHELF_INDEX), "row vanished"))?;
        if existing.section != row.section {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "install a skill",
                format!(
                    "`{}` is already shelved under {}",
                    row.name, existing.section
                ),
            )
            .with_recovery("use another name, or keep one name in one section"));
        }
        if existing == &row {
            return Ok(());
        }
        let existing = rows
            .get_mut(at)
            .ok_or_else(|| index_error(&root.join(kernel::layout::SHELF_INDEX), "row vanished"))?;
        *existing = row;
    } else {
        rows.push(row);
    }
    write_index(root, &rows)
}

pub(super) fn write_index(root: &Path, rows: &[ShelfRow]) -> Result<(), AxError> {
    validate_rows(rows)
        .map_err(|why| index_error(&root.join(kernel::layout::SHELF_INDEX), &why))?;
    std::fs::create_dir_all(root).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "create a skill shelf",
            format!("{}: {err}", root.display()),
        )
        .with_recovery("make the shelf directory writable, then install again")
    })?;
    let document = ShelfDocument {
        version: SHELF_VERSION,
        skill: rows.to_vec(),
    };
    let text = toml::to_string(&document).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "write a skill shelf index",
            err.to_string(),
        )
        .with_recovery("use only plain names and digest strings in the index")
    })?;
    crate::document::replace(&root.join(kernel::layout::SHELF_INDEX), text.as_bytes())
}

fn validate_rows(rows: &[ShelfRow]) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for row in rows {
        if !plain_name(&row.section) || !plain_name(&row.name) {
            return Err(format!(
                "{}/{} is not a plain shelf name",
                row.section, row.name
            ));
        }
        if !names.insert(&row.name) {
            return Err(format!("`{}` occurs more than once", row.name));
        }
    }
    Ok(())
}

fn index_error(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a skill shelf index",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("restore the index from verified CAS objects, then scan again")
}

pub fn disclosure(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .trim_start_matches(['#', ' '])
        .to_owned()
}
