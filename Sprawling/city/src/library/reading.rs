// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, B3Hash, SkillManifest};

use crate::config_layers::SHELVES_KEY;

use super::ShelfKey;
use super::shelf::{Holding, HoldingSource, OwnShelf, Shelf};

pub(super) use kernel::layout::SKILL_FILE;

pub(super) fn shelve(
    city_root: &Path,
    shelf: &OwnShelf,
    holdings: &mut BTreeMap<ShelfKey, Holding>,
    read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
) -> Result<(), AxError> {
    let root = shelf.root(&CityLayout::new(city_root));
    for row in super::shelf::read_index(&root)? {
        let manifest = read_manifest(read, &row.digest)?;
        let document = manifest.document().ok_or_else(|| {
            AxError::failure(
                AxCode::CasCorrupt,
                "read a skill from a shelf",
                format!("{}/{} has no {}", row.section, row.name, SKILL_FILE),
            )
            .with_recovery("reinstall the skill through the city's install door")
        })?;
        let text = read_text(read, document)?;
        holdings.insert(
            ShelfKey::of(&row.name),
            Holding::of(HoldingSource {
                name: row.name,
                section: row.section,
                text: &text,
                shelf: shelf.as_shelf(),
                hash: row.digest,
            }),
        );
    }
    Ok(())
}

pub fn skill_document(
    read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
    digest: &B3Hash,
    name: &str,
) -> Result<String, AxError> {
    let manifest = read_manifest(read, digest)?;
    let document = manifest.document().ok_or_else(|| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a skill from a shelf",
            format!("{name} has no SKILL.md in its admitted manifest"),
        )
        .with_recovery("reinstall the skill through the city's install door")
    })?;
    read_text(read, document)
}

pub(super) fn read_manifest(
    read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
    digest: &B3Hash,
) -> Result<SkillManifest, AxError> {
    SkillManifest::decode(&read(digest)?)
}

pub(super) fn read_text(
    read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
    digest: &B3Hash,
) -> Result<String, AxError> {
    let bytes = read(digest)?;
    String::from_utf8(bytes).map_err(|_| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a skill document",
            digest.to_string(),
        )
        .with_recovery("reinstall the skill as UTF-8 text through the install door")
    })
}

pub(super) fn shelve_external(
    index: u32,
    root: &Path,
    holdings: &mut BTreeMap<ShelfKey, Holding>,
) -> Result<(), AxError> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "mount an external skill shelf",
            format!("{} is not a directory", root.display()),
        )
        .with_recovery(format!(
            "point `{SHELVES_KEY}` at a directory, or take that entry out of the list"
        )));
    }
    for item in read_dir(root)? {
        if !item.is_dir() {
            continue;
        }
        let name = spelled(&item)?;
        if name.is_empty() {
            continue;
        }
        let document = item.join(SKILL_FILE);
        if !document.is_file() {
            continue;
        }
        let inspected = super::install::precheck::inspect(&item)?;
        let text = read_holding(&document)?;
        let path = format!("{name}/{SKILL_FILE}");
        holdings.insert(
            ShelfKey::of(&name),
            Holding::of(HoldingSource {
                name,
                section: String::new(),
                text: &text,
                shelf: Shelf::External { index, path },
                hash: inspected.hash,
            }),
        );
    }
    Ok(())
}

pub(super) fn read_holding(item: &Path) -> Result<String, AxError> {
    std::fs::read_to_string(item).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read a library holding",
            format!("{}: {err}", item.display()),
        )
        .with_recovery(
            "give this process permission to read the file named above, or \
             take it off the shelf, then scan again",
        )
    })
}

pub(super) fn spelled(path: &Path) -> Result<String, AxError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| unspellable(path))
}

fn unspellable(path: &Path) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "read the library",
        format!("{} is not named in Unicode", path.display()),
    )
    .with_recovery("rename it to letters, digits and dashes, then scan again")
}

pub(super) fn read_dir(path: &Path) -> Result<Vec<PathBuf>, AxError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the library",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "give this process permission to list the directory named above, then \
             scan again",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the library",
                format!("{}: {err}", path.display()),
            )
            .with_recovery(
                "give this process permission to list the directory named above, \
                 then scan again",
            )
        })?;
        out.push(entry.path());
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
