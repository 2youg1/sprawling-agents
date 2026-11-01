// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A skill and its package files, read only through a pinned manifest
//! (`crates/kernel/spec/Skill.lean`). Both the manifest and every file
//! are full, verified CAS reads; no filesystem path is followed.

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, SkillManifest};

use super::Found;
use crate::catalog::{Catalog, Expansion};

/// Opens a skill's `SKILL.md` through its admitted manifest.
pub(super) fn open_document(
    block_store: &Path,
    digest: &B3Hash,
    asked: &str,
) -> Result<Found, AxError> {
    let manifest = manifest(block_store, digest)?;
    let document = manifest.document().ok_or_else(|| {
        AxError::failure(
            AxCode::CasCorrupt,
            "read a skill",
            format!("{asked} has no SKILL.md in its manifest"),
        )
        .with_recovery("reinstall the skill through the city's install door")
    })?;
    Ok(Found::Text(read_text(block_store, document, asked)?))
}

/// Opens `<name>/<path>` when `name` is an admitted skill. A missing
/// file is a refusal inside this catalog entry, never a fallback to an
/// ordinary path.
pub(super) fn open_in_package(
    catalog: &Catalog,
    block_store: &Path,
    asked: &str,
) -> Option<Result<Found, AxError>> {
    let (name, inside) = asked.split_once('/')?;
    let Some(Expansion::Skill { manifest: digest }) = catalog.expand(name) else {
        return None;
    };
    Some((|| {
        if !kernel::skill::plain_package_path(inside) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read",
                format!("{asked} does not name a file inside the package"),
            )
            .with_recovery(
                "write the path inside the package as plain segments joined by `/`, with no empty, `.` or `..` segment",
            ));
        }
        let manifest = manifest(block_store, &digest)?;
        let file = manifest.file(inside).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read",
                format!("{asked} is not a file in the admitted manifest"),
            )
            .with_nearby(
                manifest
                    .items()
                    .iter()
                    .map(|item| format!("{name}/{}", item.path()))
                    .collect(),
            )
            .with_recovery("name a file the manifest lists, or read the skill's SKILL.md")
        })?;
        Ok(Found::Text(read_text(block_store, file, asked)?))
    })())
}

fn manifest(block_store: &Path, digest: &B3Hash) -> Result<SkillManifest, AxError> {
    let cas = storage::Cas::open(block_store).map_err(storage::StorageError::into_ax)?;
    let bytes = cas.get(digest).map_err(storage::StorageError::into_ax)?;
    SkillManifest::decode(&bytes)
}

fn read_text(block_store: &Path, digest: &B3Hash, asked: &str) -> Result<String, AxError> {
    let cas = storage::Cas::open(block_store).map_err(storage::StorageError::into_ax)?;
    let bytes = cas.get(digest).map_err(storage::StorageError::into_ax)?;
    String::from_utf8(bytes).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read",
            format!("{asked} is not UTF-8 text"),
        )
        .with_recovery("read hands back text; these bytes are not UTF-8")
    })
}
