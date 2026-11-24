// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions, Permissions};
use kernel::{AxCode, AxError, B3Hash};

use super::disk::{Identity, Root, conflict, failure};
use super::paths::{STAGE_PREFIX, split};

pub(super) struct Stage {
    parent: Dir,
    path: PathBuf,
    name: String,
    leaf: String,
    pub(super) id: Identity,
    version: B3Hash,
}

impl Root<'_> {
    pub(super) fn stage(
        &self,
        path: &str,
        nonce: B3Hash,
        bytes: &[u8],
        permissions: Permissions,
    ) -> Result<Stage, AxError> {
        let (parent, leaf) = split(path)?;
        let dir = self.directory(parent)?;
        let name = format!("{STAGE_PREFIX}{nonce}");
        let shown = self.path.join(path);
        let mut staged = dir
            .open_with(
                &name,
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .follow(FollowSymlinks::No),
            )
            .map_err(|err| failure("create a workspace replacement stage", &shown, &err))?;
        let emitted = staged
            .write_all(bytes)
            .and_then(|()| staged.set_permissions(permissions))
            .and_then(|()| staged.sync_all())
            .and_then(|()| staged.metadata());
        let id = match emitted {
            Ok(meta) => Identity::of(&meta),
            Err(err) => {
                drop(staged);
                remove_staged(&dir, &name, &shown)?;
                return Err(failure("stage workspace file bytes", &shown, &err));
            }
        };
        Ok(Stage {
            parent: dir,
            path: shown,
            name,
            leaf: leaf.to_owned(),
            id,
            version: B3Hash::digest(bytes),
        })
    }

    pub(super) fn discard_stage(
        &self,
        path: &str,
        nonce: B3Hash,
        expected: &Identity,
    ) -> Result<(), AxError> {
        let (parent, _) = split(path)?;
        let dir = match self.directory(parent) {
            Ok(dir) => dir,
            Err(error) if error.code() == &AxCode::PathNotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let stage = format!("{STAGE_PREFIX}{nonce}");
        let shown = self.path.join(path);
        match self.file(&dir, &stage, &shown) {
            Ok((file, meta)) if &Identity::of(&meta) == expected => {
                drop(file);
                remove_staged(&dir, &stage, &shown)
            }
            Ok(_) => Err(conflict(
                &shown,
                "the unfinished replacement stage acquired a different identity",
            )),
            Err(error) if error.code() == &AxCode::PathNotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

impl Stage {
    pub(super) fn replace(
        &self,
        root: &Root<'_>,
        relative: &str,
        expected: &Identity,
        baseline: B3Hash,
    ) -> Result<Identity, AxError> {
        let (parent, _) = split(relative)?;
        let current_parent = root.directory(parent)?;
        let current_parent = dir_identity(&current_parent, &self.path)?;
        let stage_parent = dir_identity(&self.parent, &self.path)?;
        if current_parent != stage_parent {
            return Err(conflict(
                &self.path,
                "the destination directory moved before the replacement",
            ));
        }
        let prepared = root.read_opened(&self.parent, &self.name, &self.path)?;
        if prepared.id != self.id || prepared.version != self.version {
            return Err(conflict(
                &self.path,
                "the staged version changed before the replacement",
            ));
        }
        let actual = root.read_opened(&self.parent, &self.leaf, &self.path)?;
        if &actual.id != expected || actual.version != baseline {
            return Err(conflict(
                &self.path,
                &format!(
                    "expected baseline {baseline}, observed {}; the save was not applied",
                    actual.version
                ),
            ));
        }
        self.parent
            .rename(&self.name, &self.parent, &self.leaf)
            .map_err(|err| failure("replace the workspace file", &self.path, &err))?;
        self.landed(root).map_err(|error| {
            AxError::failure(
                AxCode::StorageFatal,
                "confirm the replacement is durable",
                format!("{}: {}", self.path.display(), error.subject()),
            )
            .effect_unknown()
            .with_recovery("reopen the file and check its version before retrying the save")
        })
    }

    fn landed(&self, root: &Root<'_>) -> Result<Identity, AxError> {
        sync_directory(&self.parent)
            .map_err(|err| failure("flush the updated workspace directory", &self.path, &err))?;
        let (_, meta) = root.file(&self.parent, &self.leaf, &self.path)?;
        let identity = Identity::of(&meta);
        if !identity.matches_stage(&self.id) {
            return Err(conflict(
                &self.path,
                "the file changed immediately after the replacement landed",
            ));
        }
        Ok(identity)
    }

    pub(super) fn discard(&self) -> Result<(), AxError> {
        remove_staged(&self.parent, &self.name, &self.path)
    }
}

fn dir_identity(directory: &Dir, path: &Path) -> Result<Identity, AxError> {
    directory
        .dir_metadata()
        .map(|meta| Identity::of(&meta))
        .map_err(|err| failure("verify the replacement directory", path, &err))
}

#[cfg(windows)]
fn sync_directory(_directory: &Dir) -> io::Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(directory: &Dir) -> io::Result<()> {
    directory.open(".")?.sync_all()
}

fn remove_staged(dir: &Dir, stage: &str, shown: &Path) -> Result<(), AxError> {
    match dir.remove_file(stage) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(failure("remove workspace replacement stage", shown, &err)),
    }
}
