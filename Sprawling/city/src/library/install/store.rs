// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, B3Hash, SkillManifest};

use super::super::reading;
use super::super::shelf::{holding_file_name, plain_name};
use super::precheck::{Item, Shape, inspect};
use super::{Admitted, Shelved, Slot};

pub(super) struct Snapshot<'a> {
    pub(super) shape: &'a Shape,
    pub(super) stored: &'a [u8],
    pub(super) manifest: &'a SkillManifest,
    pub(super) hash: B3Hash,
}

pub(super) fn store_snapshot(
    snapshot: Snapshot<'_>,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Admitted, AxError> {
    match snapshot.shape {
        Shape::Document => {
            let expected = snapshot.manifest.document().ok_or_else(|| {
                store_disagrees(&snapshot.hash, &B3Hash::digest(b"missing document"))
            })?;
            let stored = register(snapshot.stored)?;
            if &stored != expected {
                return Err(store_disagrees(expected, &stored));
            }
        }
        Shape::Package(items) => {
            for item in items {
                let Item::File(path, range) = item else {
                    continue;
                };
                let expected = snapshot.manifest.file(path).ok_or_else(|| {
                    store_disagrees(&snapshot.hash, &B3Hash::digest(path.as_bytes()))
                })?;
                let bytes = snapshot
                    .stored
                    .get(range.clone())
                    .ok_or_else(|| store_disagrees(expected, &B3Hash::digest(path.as_bytes())))?;
                let stored = register(bytes)?;
                if &stored != expected {
                    return Err(store_disagrees(expected, &stored));
                }
            }
        }
    }
    let manifest = snapshot.manifest.encode()?;
    let stored = register(&manifest)?;
    if stored != snapshot.hash {
        return Err(store_disagrees(&snapshot.hash, &stored));
    }
    Ok(Admitted {
        hash: stored,
        manifest: snapshot.manifest.clone(),
    })
}

pub(super) fn document_text(shape: &Shape, stored: &[u8]) -> Result<String, AxError> {
    let bytes = match shape {
        Shape::Document => Some(stored),
        Shape::Package(items) => items.iter().find_map(|item| match item {
            Item::File(path, range) if path == reading::SKILL_FILE => stored.get(range.clone()),
            Item::File(..) | Item::Directory(_) => None,
        }),
    };
    let bytes = bytes.ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read an installed skill",
            "the manifest has no SKILL.md",
        )
        .with_recovery("reinstall the skill with a UTF-8 SKILL.md")
    })?;
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read an installed skill",
            "SKILL.md is not UTF-8",
        )
        .with_recovery("reinstall the skill with a UTF-8 SKILL.md")
    })
}

/// What the shelf holds under `name`, distinguishing the current index
/// from the legacy directory that migration may replace.
pub(super) fn shelf_state(slot: &Slot, city_root: &Path, name: &str) -> Result<Shelved, AxError> {
    let root = slot.shelf.root(&CityLayout::new(city_root));
    if let Some(row) = super::super::shelf::read_index(&root)?
        .into_iter()
        .find(|row| row.name == name)
    {
        if row.section != slot.section {
            return Err(name_taken(name, &row.section));
        }
        if legacy_shelf_state(slot, &root, name)?.is_some() {
            return Err(name_taken(name, "both an index row and a legacy directory"));
        }
        return Ok(Shelved::Current(row.digest));
    }
    let Some(legacy) = legacy_shelf_state(slot, &root, name)? else {
        return Ok(Shelved::Empty);
    };
    Ok(Shelved::Legacy(legacy))
}

pub(super) fn legacy_shelf_state(
    slot: &Slot,
    root: &Path,
    name: &str,
) -> Result<Option<B3Hash>, AxError> {
    if !root.exists() {
        return Ok(None);
    }
    for section in reading::read_dir(root)? {
        if !section.is_dir() {
            continue;
        }
        let held_under = reading::spelled(&section)?;
        if !plain_name(&held_under) {
            continue;
        }
        let held: Vec<_> = [section.join(holding_file_name(name)), section.join(name)]
            .into_iter()
            .filter(|named| named.exists())
            .collect();
        match held.as_slice() {
            [] => {}
            [_, _, ..] => {
                let file = holding_file_name(name);
                let twice = format!("{held_under} twice, as {file} and {name}/");
                return Err(name_taken(name, &twice));
            }
            [_] if held_under != slot.section => return Err(name_taken(name, &held_under)),
            [one] => return Ok(Some(inspect(one)?.source_hash)),
        }
    }
    Ok(None)
}

pub(super) fn require_plain(what: &str, role: &str) -> Result<(), AxError> {
    if plain_name(what) {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("{role} is `{what}`"),
    )
    .with_recovery(
        "name it with letters, digits and dashes: one plain name, no leading dot, no path separators",
    ))
}

pub(super) fn name_taken(name: &str, held_under: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("`{name}` is already shelved under {held_under}"),
    )
    .with_recovery(format!(
        "install it under another name, or take the shelved `{name}` off the shelf first: one name holds one holding"
    ))
}

pub(super) fn moved_underneath(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "install a skill",
        format!(
            "{} changed while this skill was being installed: {why}",
            path.display()
        ),
    )
    .with_recovery("start the install again from what is there now")
}

pub(super) fn store_disagrees(ours: &B3Hash, stored: &B3Hash) -> AxError {
    AxError::failure(
        AxCode::CasCorrupt,
        "register an installed skill",
        format!("the store answered {stored} for content that hashes to {ours}"),
    )
    .with_recovery("check the store this install registers with before installing again")
}

pub(super) fn legacy_io(path: &Path, err: std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "remove a migrated skill directory",
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "the index row already names the new version; make the old shelf writable and remove \
         it by hand",
    )
}
