// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::{OsStr, OsString};
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::alias::WriteTarget;
use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

pub(crate) enum Bits<'a> {

    Of(&'a Path),

    OfReplaced,
}

pub(crate) fn land(
    vfs: &mut dyn Vfs,
    target: WriteTarget,
    bytes: &[u8],
    bits: Bits<'_>,
) -> Result<(), StorageError> {
    let path = target.as_path();
    let staged = staging_path(path)?;
    if vfs.exists(&staged) {
        vfs.remove_file(&staged)
            .map_err(io_err("clear a stale bundle staging file", &staged))?;
    }
    vfs.append(&staged, bytes)
        .map_err(io_err("stage a bundle file", &staged))?;
    vfs.sync_data(&staged)
        .map_err(io_err("flush a bundle file", &staged))?;
    let original = match bits {
        Bits::Of(source) => Some(source),
        Bits::OfReplaced => vfs.exists(path).then_some(path),
    };
    if let Some(original) = original {
        vfs.copy_permissions(original, &staged)
            .map_err(io_err("keep a bundle file's permissions", original))?;
    }
    vfs.rename(&staged, path)
        .map_err(io_err("replace a bundle file", path))?;
    match path.parent() {
        Some(dir) => vfs
            .sync_dir(dir)
            .map_err(io_err("record a bundle file's name", dir)),
        None => Ok(()),
    }
}

pub(crate) fn create(target: WriteTarget, bytes: &[u8]) -> Result<(), StorageError> {
    let path = target.as_path();
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            return Err(StorageError::NameTaken {
                path: path.to_path_buf(),
            });
        }
        Err(err) => return Err(io_err("create a file", path)(err)),
    };
    let written = file.write_all(bytes).and_then(|()| file.sync_data());
    drop(file);
    if let Err(err) = written {

        std::fs::remove_file(path).map_err(io_err("take back a half-created file", path))?;
        return Err(io_err("create a file", path)(err));
    }
    match path.parent() {
        Some(dir) => RealFs::new()
            .sync_dir(dir)
            .map_err(io_err("record a created file's name", dir)),
        None => Ok(()),
    }
}

fn staging_path(path: &Path) -> Result<PathBuf, StorageError> {
    let name = path.file_name().ok_or_else(|| {
        io_err("stage a bundle file", path)(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a bundle file needs a file name of its own",
        ))
    })?;
    let mut staged = OsString::from(STAGING_PREFIX);
    staged.push(name);
    staged.push(STAGING_SUFFIX);
    Ok(path.with_file_name(staged))
}

pub(crate) fn is_staging_name(name: &OsStr) -> bool {
    let spelled = name.as_encoded_bytes();
    spelled.len() > STAGING_PREFIX.len().saturating_add(STAGING_SUFFIX.len())
        && spelled.starts_with(STAGING_PREFIX.as_bytes())
        && spelled.ends_with(STAGING_SUFFIX.as_bytes())
}

const STAGING_PREFIX: &str = ".";
const STAGING_SUFFIX: &str = ".part";
