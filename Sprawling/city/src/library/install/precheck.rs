// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fs::{File, Metadata};
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, SkillItem, SkillManifest};

use crate::library::reading;
use crate::library::shelf::{HOLDING_EXT, holding_name};

pub(super) mod walk;

const PACKAGE_HEADER: &[u8] = b"sprawling-skill-package/1\n";

pub(in crate::library) struct Inspected {
    pub(in crate::library) name: String,
    pub(in crate::library) shape: Shape,

    pub(in crate::library) stored: Vec<u8>,
    pub(in crate::library) source_hash: B3Hash,
    pub(in crate::library) manifest: SkillManifest,

    pub(in crate::library) hash: B3Hash,
}

#[derive(Debug, Clone)]
pub(in crate::library) enum Shape {

    Document,

    Package(Vec<Item>),
}

#[derive(Debug, Clone)]
pub(in crate::library) enum Item {
    Directory(String),
    File(String, Range<usize>),
}

pub(in crate::library) fn inspect(path: &Path) -> Result<Inspected, AxError> {
    let meta = unlinked(path)?;
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

fn unlinked(path: &Path) -> Result<Metadata, AxError> {
    let meta = std::fs::symlink_metadata(path).map_err(|err| source_io(path, &err))?;
    if meta.file_type().is_symlink() {
        return Err(refuses_link(path));
    }
    Ok(meta)
}

fn inspect_package(dir: &Path) -> Result<Inspected, AxError> {
    let mut found = walk::items(dir)?;
    found.sort_by(|left, right| left.path.cmp(&right.path));
    let mut stored = PACKAGE_HEADER.to_vec();
    let mut items = Vec::with_capacity(found.len());
    for walk::Found { path, bytes } in found {
        items.push(append_item(&mut stored, path, bytes.as_deref(), dir)?);
    }
    let document = items.iter().find_map(|item| match item {
        Item::File(path, bytes) if path == reading::SKILL_FILE => Some(bytes.clone()),
        Item::File(..) | Item::Directory(_) => None,
    });
    let Some(document) = document else {
        return Err(not_a_source(
            dir,
            &format!("it holds no {}", reading::SKILL_FILE),
        ));
    };
    require_text(&dir.join(reading::SKILL_FILE), stored.get(document))?;
    let shape = Shape::Package(items);
    let manifest = manifest_of(&shape, &stored)?;
    let hash = B3Hash::digest(&manifest.encode()?);
    let source_hash = B3Hash::digest(&stored);
    Ok(Inspected {
        name: reading::spelled(dir)?,
        shape,
        stored,
        source_hash,
        manifest,
        hash,
    })
}

pub(super) fn manifest_of(shape: &Shape, stored: &[u8]) -> Result<SkillManifest, AxError> {
    let items = match shape {
        Shape::Document => vec![SkillItem::File {
            path: reading::SKILL_FILE.to_owned(),
            digest: B3Hash::digest(stored),
        }],
        Shape::Package(items) => items
            .iter()
            .map(|item| match item {
                Item::Directory(path) => Ok(SkillItem::Directory {
                    path: path.trim_end_matches('/').to_owned(),
                }),
                Item::File(path, range) => {
                    let bytes = stored.get(range.clone()).ok_or_else(|| {
                        not_a_source(Path::new(path), "a file range left the source snapshot")
                    })?;
                    Ok(SkillItem::File {
                        path: path.clone(),
                        digest: B3Hash::digest(bytes),
                    })
                }
            })
            .collect::<Result<Vec<_>, AxError>>()?,
    };
    SkillManifest::new(items)
}

fn append_item(
    stored: &mut Vec<u8>,
    path: String,
    bytes: Option<&[u8]>,
    package: &Path,
) -> Result<Item, AxError> {
    stored.push(if bytes.is_some() { b'f' } else { b'd' });
    append_len(stored, path.len(), package)?;
    stored.extend_from_slice(path.as_bytes());
    let Some(bytes) = bytes else {
        return Ok(Item::Directory(path));
    };
    append_len(stored, bytes.len(), package)?;
    let start = stored.len();
    stored.extend_from_slice(bytes);
    Ok(Item::File(path, start..stored.len()))
}

fn append_len(stored: &mut Vec<u8>, len: usize, entry: &Path) -> Result<(), AxError> {
    let len = u64::try_from(len).map_err(|_| not_a_source(entry, "it is too large to file"))?;
    stored.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn inspect_document(file: &Path) -> Result<Inspected, AxError> {
    let spelled = reading::spelled(file)?;
    let Some(name) = holding_name(&spelled) else {
        return Err(not_a_source(
            file,
            &format!("a resident's skill is one .{HOLDING_EXT} document"),
        ));
    };
    let stored = read_unlinked(file, &unlinked(file)?)?;
    require_text(file, Some(&stored))?;
    let shape = Shape::Document;
    let manifest = manifest_of(&shape, &stored)?;
    let hash = B3Hash::digest(&manifest.encode()?);
    let source_hash = B3Hash::digest(&stored);
    Ok(Inspected {
        name,
        shape,
        stored,
        source_hash,
        manifest,
        hash,
    })
}

fn read_unlinked(path: &Path, judged: &Metadata) -> Result<Vec<u8>, AxError> {
    let mut file = open_unfollowed(path)?;
    let opened = file.metadata().map_err(|err| source_io(path, &err))?;
    if opened.file_type().is_symlink() || !same_file(judged, &opened) {
        return Err(refuses_link(path));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| source_io(path, &err))?;
    Ok(bytes)
}

#[cfg(windows)]
fn open_unfollowed(path: &Path) -> Result<File, AxError> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|err| source_io(path, &err))
}

#[cfg(not(windows))]
fn open_unfollowed(path: &Path) -> Result<File, AxError> {
    File::open(path).map_err(|err| source_io(path, &err))
}

#[cfg(unix)]
fn same_file(judged: &Metadata, opened: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    judged.dev() == opened.dev() && judged.ino() == opened.ino()
}

#[cfg(windows)]
fn same_file(_judged: &Metadata, opened: &Metadata) -> bool {
    opened.is_file() && plain(opened)
}

#[cfg(windows)]
pub(super) fn plain(opened: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    opened.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
}

#[cfg(not(windows))]
pub(super) fn plain(opened: &Metadata) -> bool {
    !opened.file_type().is_symlink()
}

fn require_text(path: &Path, bytes: Option<&[u8]>) -> Result<(), AxError> {
    match bytes.map(std::str::from_utf8) {
        Some(Ok(_)) => Ok(()),
        Some(Err(_)) | None => Err(not_a_source(path, "it is not UTF-8 text")),
    }
}

pub(super) fn refuses_link(path: &Path) -> AxError {
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

pub(super) fn not_a_source(path: &Path, why: &str) -> AxError {
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

pub(super) fn source_io(path: &Path, err: &std::io::Error) -> AxError {
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
