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
//! [`reading::SKILL_FILE`] and whatever else its author put beside it,
//! and a resident's own skill is one `.md` document. A link anywhere in
//! a source is refused whatever it points at: the bytes a link hands
//! over are not the package's to file, and where it points can change
//! between this check and the landing.
//!
//! The reading keeps every byte it read - one snapshot per item - and
//! hashes exactly those bytes, so the landing writes what was judged and
//! the recheck before it compares one hash that covers every item.

use std::fs::{File, Metadata};
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use kernel::{AxCode, AxError, B3Hash};

use crate::library::reading;
use crate::library::shelf::{HOLDING_EXT, holding_name};

/// The first bytes of a package's canonical string, so a document that
/// happens to read like one is never taken for a package in the store.
const PACKAGE_HEADER: &[u8] = b"sprawling-skill-package/1\n";

/// What a static precheck read: the name, the shape it lands in, the
/// bytes the hash is taken over, and that hash.
pub(super) struct Inspected {
    pub(super) name: String,
    pub(super) shape: Shape,
    /// The document's bytes, or the package's canonical string: what
    /// the store is handed and what [`Inspected::hash`] is taken over.
    pub(super) stored: Vec<u8>,
    pub(super) hash: B3Hash,
}

/// The two shapes a source arrives in, each landing as the shape the
/// scan reads back.
#[derive(Debug, Clone)]
pub(super) enum Shape {
    /// One document, landing as `<name>.md`; its bytes are `stored`.
    Document,
    /// A package, landing as `<name>/`: every item in canonical order,
    /// a file's bytes held as a range of `stored`.
    Package(Vec<Item>),
}

/// One item of a package, by its path relative to the package with
/// segments joined by `/`.
#[derive(Debug, Clone)]
pub(super) enum Item {
    Directory(String),
    File(String, Range<usize>),
}

/// Reads and judges one source without executing anything in it.
///
/// # Errors
/// Refuses a source that is not there (`E_PATH_NOT_FOUND`); a source
/// that is not one skill document or package (`E_INVALID_ARGS`); and a
/// link anywhere in it (`E_INVALID_ARGS`, whatever it points at).
pub(super) fn inspect(path: &Path) -> Result<Inspected, AxError> {
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

/// The one judgement of "no link": the shape of `path` itself, never of
/// what it points at. Every item a precheck reads passes through here.
fn unlinked(path: &Path) -> Result<Metadata, AxError> {
    let meta = std::fs::symlink_metadata(path).map_err(|err| source_io(path, &err))?;
    if meta.file_type().is_symlink() {
        return Err(refuses_link(path));
    }
    Ok(meta)
}

/// Reads a package whole: every item under it, in canonical order, and
/// [`reading::SKILL_FILE`] among them as text the scan can read.
fn inspect_package(dir: &Path) -> Result<Inspected, AxError> {
    let mut found = Vec::new();
    let mut open = vec![(dir.to_path_buf(), String::new())];
    while let Some((at, prefix)) = open.pop() {
        for entry in reading::read_dir(&at)? {
            let path = format!("{prefix}{}", reading::spelled(&entry)?);
            let meta = unlinked(&entry)?;
            if meta.is_dir() {
                open.push((entry.clone(), format!("{path}/")));
            } else if !meta.is_file() {
                return Err(not_a_source(&entry, "it is neither a directory nor a file"));
            }
            found.push((path, entry, meta));
        }
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    let mut stored = PACKAGE_HEADER.to_vec();
    let mut items = Vec::with_capacity(found.len());
    for (path, entry, meta) in found {
        items.push(append_item(&mut stored, path, &entry, &meta)?);
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
    let hash = B3Hash::digest(&stored);
    Ok(Inspected {
        name: reading::spelled(dir)?,
        shape: Shape::Package(items),
        stored,
        hash,
    })
}

/// Appends one item to the canonical string: its kind, its
/// length-prefixed path and, for a file, its length-prefixed bytes.
fn append_item(
    stored: &mut Vec<u8>,
    path: String,
    entry: &Path,
    meta: &Metadata,
) -> Result<Item, AxError> {
    let tag = if meta.is_dir() { b'd' } else { b'f' };
    stored.push(tag);
    append_len(stored, path.len(), entry)?;
    stored.extend_from_slice(path.as_bytes());
    if meta.is_dir() {
        return Ok(Item::Directory(path));
    }
    let bytes = read_unlinked(entry, meta)?;
    append_len(stored, bytes.len(), entry)?;
    let start = stored.len();
    stored.extend_from_slice(&bytes);
    Ok(Item::File(path, start..stored.len()))
}

fn append_len(stored: &mut Vec<u8>, len: usize, entry: &Path) -> Result<(), AxError> {
    let len = u64::try_from(len).map_err(|_| not_a_source(entry, "it is too large to file"))?;
    stored.extend_from_slice(&len.to_le_bytes());
    Ok(())
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
    let stored = read_unlinked(file, &unlinked(file)?)?;
    require_text(file, Some(&stored))?;
    let hash = B3Hash::digest(&stored);
    Ok(Inspected {
        name,
        shape: Shape::Document,
        stored,
        hash,
    })
}

/// Reads a file judged not to be a link, and refuses when what the open
/// reached is not that file: a path swapped for a link between the
/// judgement and the open hands over bytes from somewhere else.
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

/// Windows opens the link itself rather than what it points at, so the
/// handle's own metadata says whether the path was a link at the open.
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

/// Whether the opened file is the one judged: the inode on unix, where
/// an open follows a link placed after the judgement.
#[cfg(unix)]
fn same_file(judged: &Metadata, opened: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    judged.dev() == opened.dev() && judged.ino() == opened.ino()
}

/// On Windows the open did not follow a link, so a plain file behind the
/// handle is the path's own file.
#[cfg(not(unix))]
fn same_file(_judged: &Metadata, opened: &Metadata) -> bool {
    opened.is_file()
}

/// The scan reads a skill document as text, so a document it could not
/// read is refused here rather than left to fail every later scan.
fn require_text(path: &Path, bytes: Option<&[u8]>) -> Result<(), AxError> {
    match bytes.map(std::str::from_utf8) {
        Some(Ok(_)) => Ok(()),
        Some(Err(_)) | None => Err(not_a_source(path, "it is not UTF-8 text")),
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
