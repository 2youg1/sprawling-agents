// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The static precheck: reading one skill source and judging its shape,
//! without executing, compiling or interpreting anything in it
//! (city-SPEC.md section 8-28).
//!
//! A source arrives in one of two shapes and is read into one value: a
//! package is a directory named after the skill holding
//! [`reading::SKILL_FILE`], and a resident's own skill is one `.md`
//! document. A package reached through a link is refused whatever the
//! link points at: the bytes a link hands over are not the package's to
//! file, and where it points can change between this check and the
//! landing.
//!
//! The reading also records the shape it read: every entry with its
//! length, and the document's content hash. The landing rechecks
//! against that record, so a source that moved underneath the install
//! is refused before anything lands.

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash};

use crate::library::reading;
use crate::library::shelf::{HOLDING_EXT, holding_name};

/// What a static precheck read: the name, the document, its hash, and
/// the shape it read them in.
pub(super) struct Inspected {
    pub(super) name: String,
    pub(super) body: String,
    pub(super) hash: B3Hash,
    pub(super) fingerprint: Fingerprint,
}

/// One source's shape at one moment: every entry with its length. An
/// entry appearing, disappearing or changing size is the source being
/// swapped, even before its bytes are compared. The rules of shape
/// themselves - one document, no links - are judged again from scratch
/// by the recheck, so a difference this record cannot hold still
/// refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Fingerprint(Vec<(String, u64)>);

/// Reads and judges one source without executing anything in it.
///
/// # Errors
/// Refuses a source that is not there (`E_PATH_NOT_FOUND`); a source
/// that is not one skill document (`E_INVALID_ARGS`); and a package
/// reached through a link (`E_INVALID_ARGS`, whatever the link points
/// at).
pub(super) fn inspect(path: &Path) -> Result<Inspected, AxError> {
    let meta = std::fs::symlink_metadata(path).map_err(|err| source_io(path, &err))?;
    if meta.file_type().is_symlink() {
        return Err(refuses_link(path));
    }
    if meta.is_dir() {
        inspect_package(path)
    } else if meta.is_file() {
        inspect_document(path)
    } else {
        Err(not_a_source(
            path,
            "it is neither a directory nor a document",
        ))
    }
}

/// Reads a package: one entry, [`reading::SKILL_FILE`], and no link.
fn inspect_package(dir: &Path) -> Result<Inspected, AxError> {
    let mut fingerprint = Vec::new();
    let mut document = None;
    for entry in reading::read_dir(dir)? {
        let meta = std::fs::symlink_metadata(&entry).map_err(|err| source_io(&entry, &err))?;
        if meta.file_type().is_symlink() {
            return Err(refuses_link(&entry));
        }
        let file = reading::spelled(&entry)?;
        if file == reading::SKILL_FILE {
            document = Some(entry);
        }
        fingerprint.push((file, meta.len()));
    }
    let Some(document) = document else {
        return Err(not_a_source(
            dir,
            &format!("it holds no {}", reading::SKILL_FILE),
        ));
    };
    if fingerprint.len() != 1 {
        return Err(not_a_source(
            dir,
            "a package carries one skill document, and this one carries more",
        ));
    }
    let body = reading::read_holding(&document)?;
    let name = reading::spelled(dir)?;
    Ok(inspected(name, body, Fingerprint(fingerprint)))
}

/// Reads one document a resident wrote.
fn inspect_document(file: &Path) -> Result<Inspected, AxError> {
    let spelled = reading::spelled(file)?;
    let Some(name) = holding_name(&spelled) else {
        return Err(not_a_source(
            file,
            &format!("a resident's skill is one .{HOLDING_EXT} document"),
        ));
    };
    let body = reading::read_holding(file)?;
    let len = std::fs::symlink_metadata(file)
        .map_err(|err| source_io(file, &err))?
        .len();
    Ok(inspected(name, body, Fingerprint(vec![(spelled, len)])))
}

/// The value every reading becomes: the content hash is derived from
/// the bytes in hand here and nowhere else.
fn inspected(name: String, body: String, fingerprint: Fingerprint) -> Inspected {
    let hash = B3Hash::digest(body.as_bytes());
    Inspected {
        name,
        body,
        hash,
        fingerprint,
    }
}

fn refuses_link(path: &Path) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!(
            "{} is a link, and a package reached through a link is refused",
            path.display()
        ),
    )
    .with_recovery(
        "rebuild the package out of plain files and directories - a link is refused \
         whatever it points at - then install again",
    )
}

fn not_a_source(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("{}: {why}", path.display()),
    )
    .with_recovery(format!(
        "point the install at one skill package (a directory holding {}) or at one \
         .{HOLDING_EXT} document",
        reading::SKILL_FILE,
    ))
}

fn source_io(path: &Path, err: &std::io::Error) -> AxError {
    if err.kind() == std::io::ErrorKind::NotFound {
        return AxError::failure(
            AxCode::PathNotFound,
            "install a skill",
            format!("{} is not there", path.display()),
        )
        .with_recovery("point the install at a package or document that exists");
    }
    AxError::failure(
        AxCode::StorageFatal,
        "install a skill",
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "give this process permission to read the source named above, then install again",
    )
}
