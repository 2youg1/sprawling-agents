// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kernel::{AxCode, AxError};

pub(crate) fn replace(path: &Path, body: &[u8]) -> Result<(), AxError> {
    edit(path, |held| held.replace(body))
}

pub fn edit<T>(
    path: &Path,
    act: impl FnOnce(&Held<'_>) -> Result<T, AxError>,
) -> Result<T, AxError> {
    let slot = slot(path);

    let _guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    act(&Held { path })
}

pub fn edit_against(path: &Path, base: &[u8], body: &[u8]) -> Result<(), AxError> {
    revise(path, |held, on_disk| {
        if on_disk != base {
            return Err(moved(path));
        }
        held.replace(body)
    })
}

pub(crate) fn moved(path: &Path) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "replace a document",
        format!(
            "{}: the file is no longer the text the writer started from",
            path.display()
        ),
    )
    .with_recovery("read the file again and send the change once more")
}

pub fn revise<T>(
    path: &Path,
    act: impl FnOnce(&Held<'_>, &[u8]) -> Result<T, AxError>,
) -> Result<T, AxError> {
    edit(path, |held| {
        let on_disk = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(storage(path, err.to_string())),
        };
        act(held, &on_disk)
    })
}

pub struct Held<'a> {
    path: &'a Path,
}

impl Held<'_> {
    pub fn replace(&self, body: &[u8]) -> Result<(), AxError> {
        let dir = self.path.parent().ok_or_else(|| {
            storage(
                self.path,
                "a document needs a directory to sit in".to_owned(),
            )
        })?;
        std::fs::create_dir_all(dir).map_err(|err| storage(dir, err.to_string()))?;
        let staged = staging_path(self.path)?;
        stage(&staged, body)?;

        std::fs::rename(&staged, self.path).map_err(|err| {
            storage(
                self.path,
                format!("{} could not take its place: {err}", staged.display()),
            )
        })?;
        settle(dir)
    }

    pub fn remove(&self) -> Result<(), AxError> {
        std::fs::remove_file(self.path).map_err(|err| storage(self.path, err.to_string()))
    }
}

fn stage(staged: &Path, body: &[u8]) -> Result<(), AxError> {
    let mut file = std::fs::File::create(staged).map_err(|err| storage(staged, err.to_string()))?;
    file.write_all(body)
        .map_err(|err| storage(staged, err.to_string()))?;
    file.sync_all()
        .map_err(|err| storage(staged, err.to_string()))
}

#[cfg(unix)]
fn settle(dir: &Path) -> Result<(), AxError> {
    std::fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|err| storage(dir, err.to_string()))
}

#[cfg(not(unix))]
fn settle(_dir: &Path) -> Result<(), AxError> {
    Ok(())
}

fn staging_path(path: &Path) -> Result<PathBuf, AxError> {
    let name = path
        .file_name()
        .ok_or_else(|| storage(path, "a document needs a file name of its own".to_owned()))?;
    Ok(path.with_file_name(kernel::layout::document_staging_name(name)))
}

fn slot(path: &Path) -> Arc<Mutex<()>> {
    static WRITERS: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let table = WRITERS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        table
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

fn storage(path: &Path, why: String) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "replace a document",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("make the directory writable and check the disk has room, then save again")
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
