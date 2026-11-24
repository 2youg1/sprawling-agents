// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{self, Read as _};
use std::path::Path;
use std::time::UNIX_EPOCH;

use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, Metadata, OpenOptions, Permissions};
use kernel::{AxCode, AxError, B3Hash};
use serde::{Deserialize, Serialize};
use wire::{WorkspaceEntry, WorkspaceEntryKind};

const READ_MAX: u64 = 256 * 1024 * 1024;
const RECOVERY_SCAN_MAX: usize = 100_000;
use super::paths::{joined, split, usable_name};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Identity {
    volume: u64,
    file: u64,
    born: Option<(u64, u32)>,
}

impl Identity {
    pub(super) fn of(meta: &Metadata) -> Self {
        let born = match meta.created() {
            Ok(found) => match found.into_std().duration_since(UNIX_EPOCH) {
                Ok(elapsed) => Some((elapsed.as_secs(), elapsed.subsec_nanos())),
                Err(_) => None,
            },
            Err(_) => None,
        };
        Self {
            volume: meta.dev(),
            file: meta.ino(),
            born,
        }
    }

    pub(super) fn matches_stage(&self, staged: &Self) -> bool {
        if cfg!(windows) {
            self.volume == staged.volume && self.file == staged.file
        } else {
            self == staged
        }
    }

    pub(super) fn has_birth(&self) -> bool {
        self.born.is_some()
    }
}

pub(super) struct Root<'a> {
    dir: Dir,
    pub(super) path: &'a Path,
    device: u64,
}

pub(super) struct Seen {
    pub(super) id: Identity,
    pub(super) version: B3Hash,
    pub(super) bytes: Vec<u8>,
    pub(super) permissions: Permissions,
}

impl<'a> Root<'a> {
    pub(super) fn open(path: &'a Path) -> Result<(Self, Identity), AxError> {
        let dir = Dir::open_ambient_dir(path, cap_std::ambient_authority())
            .map_err(|err| failure("open the workspace root", path, &err))?;
        let meta = dir
            .dir_metadata()
            .map_err(|err| failure("inspect the workspace root", path, &err))?;
        if !plain(&meta) {
            return Err(refused(path, "the root is a filesystem alias"));
        }
        let id = Identity::of(&meta);
        Ok((
            Self {
                dir,
                path,
                device: id.volume,
            },
            id,
        ))
    }

    pub(super) fn entries(&self, path: &str) -> Result<Vec<WorkspaceEntry>, AxError> {
        self.list(path).map(|mut entries| {
            entries.sort_unstable_by(|left, right| {
                rank(left.kind)
                    .cmp(&rank(right.kind))
                    .then_with(|| left.name.cmp(&right.name))
            });
            entries
        })
    }

    fn list(&self, path: &str) -> Result<Vec<WorkspaceEntry>, AxError> {
        let shown = self.path.join(path);
        let current = self.directory(path)?;
        let entries = current
            .entries()
            .map_err(|err| failure("list workspace directory", &shown, &err))?;
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| failure("list workspace directory", &shown, &err))?;
            let name = entry.file_name();
            let Some(name) = name.to_str().filter(|name| usable_name(name)) else {
                continue;
            };
            if result.len() == RECOVERY_SCAN_MAX {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "list the workspace directory",
                    format!(
                        "{} contains more than the supported 100,000 entries",
                        shown.display()
                    ),
                )
                .with_recovery("divide the directory into folders with fewer entries"));
            }
            let at = joined(path, name);
            let kind = entry
                .file_type()
                .map_err(|err| failure("inspect a workspace entry", &self.path.join(&at), &err))?;
            let kind = if kind.is_dir() {
                WorkspaceEntryKind::Directory
            } else if kind.is_file() {
                WorkspaceEntryKind::File
            } else {
                continue;
            };
            result.push(WorkspaceEntry {
                name: name.to_owned(),
                path: at,
                kind,
            });
        }
        Ok(result)
    }

    pub(super) fn read(&self, path: &str) -> Result<Seen, AxError> {
        let shown = self.path.join(path);
        let (parent, leaf) = split(path)?;
        let dir = self.directory(parent)?;
        self.read_opened(&dir, leaf, &shown)
    }

    pub(super) fn read_opened(&self, dir: &Dir, leaf: &str, shown: &Path) -> Result<Seen, AxError> {
        let (mut file, before) = self.file(dir, leaf, shown)?;
        if before.len() > READ_MAX {
            return Err(too_large(shown));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(READ_MAX.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|err| failure("read the workspace file", shown, &err))?;
        if u64::try_from(bytes.len()).is_ok_and(|size| size > READ_MAX) {
            return Err(too_large(shown));
        }
        let after = file
            .metadata()
            .map_err(|err| failure("check the read file", shown, &err))?;
        let id = Identity::of(&before);
        let (_, current) = self.file(dir, leaf, shown)?;
        if before.len() != after.len()
            || before
                .modified()
                .map_err(|err| failure("time the read file", shown, &err))?
                != after
                    .modified()
                    .map_err(|err| failure("time the read file", shown, &err))?
            || id != Identity::of(&current)
        {
            return Err(conflict(shown, "the file changed while it was being read"));
        }
        Ok(Seen {
            id,
            version: B3Hash::digest(&bytes),
            bytes,
            permissions: before.permissions(),
        })
    }

    pub(super) fn find(&self, wanted: &Identity, original: &str) -> Result<String, AxError> {
        if !wanted.has_birth() {
            return Err(conflict(
                &self.path.join(original),
                "the filesystem does not provide creation evidence to confirm a moved file",
            ));
        }
        let mut directories = vec![String::new()];
        let mut remaining = RECOVERY_SCAN_MAX;
        while let Some(path) = directories.pop() {
            let listed = match self.list(&path) {
                Ok(listed) => listed,
                Err(error)
                    if error.code() == &AxCode::PathNotFound
                        || error.code() == &AxCode::GateDenied =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            remaining = remaining.checked_sub(listed.len()).ok_or_else(|| {
                conflict(
                    &self.path.join(original),
                    "the recovery search reached its 100,000-entry limit",
                )
            })?;
            let directory = self.directory(&path)?;
            for entry in listed {
                match entry.kind {
                    WorkspaceEntryKind::Directory => directories.push(entry.path),
                    WorkspaceEntryKind::File => {
                        let shown = self.path.join(&entry.path);
                        match self.file(&directory, &entry.name, &shown) {
                            Ok((_, meta)) if &Identity::of(&meta) == wanted => {
                                return Ok(entry.path);
                            }
                            Ok(_) => {}
                            Err(error)
                                if error.code() == &AxCode::PathNotFound
                                    || error.code() == &AxCode::GateDenied => {}
                            Err(error) => return Err(error),
                        }
                    }
                }
            }
        }
        Err(conflict(
            &self.path.join(original),
            "the registered file could not be verified at its former path or elsewhere in the root",
        ))
    }

    pub(super) fn watch_point(&self, path: &str) -> Result<super::watching::WatchPoint, AxError> {
        let directory = self.directory(path)?;
        let shown = self.path.join(path);
        let meta = directory
            .dir_metadata()
            .map_err(|err| failure("inspect the watched workspace directory", &shown, &err))?;
        Ok(super::watching::WatchPoint {
            directory,
            #[cfg(not(target_os = "linux"))]
            shown,
            identity: Identity::of(&meta),
        })
    }

    pub(super) fn directory(&self, path: &str) -> Result<Dir, AxError> {
        let mut directory = self
            .dir
            .try_clone()
            .map_err(|err| failure("retain the workspace root", self.path, &err))?;
        if path.is_empty() {
            return Ok(directory);
        }
        let mut shown = self.path.to_path_buf();
        for segment in path.split('/') {
            if !usable_name(segment) {
                return Err(refused(
                    &self.path.join(path),
                    "a relative path component is protected or invalid",
                ));
            }
            shown.push(segment);
            let candidate = directory
                .symlink_metadata(segment)
                .map_err(|err| failure("locate a workspace directory", &shown, &err))?;
            if !candidate.is_dir() || !plain(&candidate) {
                return Err(refused(
                    &shown,
                    "the entry is an alias or does not name a directory",
                ));
            }
            directory = directory
                .open_dir_nofollow(segment)
                .map_err(|err| failure("open a workspace directory", &shown, &err))?;
            let meta = directory
                .dir_metadata()
                .map_err(|err| failure("inspect a workspace directory", &shown, &err))?;
            if !plain(&meta) || meta.dev() != self.device {
                return Err(refused(
                    &shown,
                    "a directory is an alias or crosses a mounted volume",
                ));
            }
        }
        Ok(directory)
    }

    pub(super) fn file(
        &self,
        parent: &Dir,
        leaf: &str,
        shown: &Path,
    ) -> Result<(File, Metadata), AxError> {
        let meta = parent
            .symlink_metadata(leaf)
            .map_err(|err| failure("inspect the workspace file", shown, &err))?;
        if !meta.is_file() || !plain(&meta) {
            return Err(refused(
                shown,
                "the entry is an alias or does not name a regular file",
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt as _;
            options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits().cast_signed());
        }
        let file = parent
            .open_with(leaf, &options)
            .map_err(|err| failure("open a workspace file", shown, &err))?;
        let meta = file
            .metadata()
            .map_err(|err| failure("inspect the opened workspace file", shown, &err))?;
        if !meta.is_file() || !plain(&meta) || meta.nlink() != 1 || meta.dev() != self.device {
            return Err(refused(
                shown,
                "the opened file is an alias, has multiple names, or crosses a mounted volume",
            ));
        }
        Ok((file, meta))
    }
}

fn rank(kind: WorkspaceEntryKind) -> u8 {
    match kind {
        WorkspaceEntryKind::Directory => 0,
        WorkspaceEntryKind::File => 1,
    }
}

#[cfg(windows)]
fn plain(meta: &Metadata) -> bool {
    use cap_std::fs::MetadataExt as _;
    const REPARSE_POINT: u32 = 0x0400;
    meta.file_attributes() & REPARSE_POINT == 0
}

#[cfg(not(windows))]
fn plain(meta: &Metadata) -> bool {
    !meta.file_type().is_symlink()
}

fn too_large(path: &Path) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read the workspace file",
        format!("{} exceeds the 256 MiB reading limit", path.display()),
    )
    .with_recovery("open a smaller document or use the operating-system file viewer")
}

pub(super) fn failure(action: &str, path: &Path, error: &io::Error) -> AxError {
    let code = if error.kind() == io::ErrorKind::NotFound {
        AxCode::PathNotFound
    } else {
        AxCode::StorageFatal
    };
    AxError::failure(code, action, format!("{}: {error}", path.display()))
        .with_recovery("check that the registered directory is mounted and that the selected path is accessible, then try again")
}

pub(super) fn refused(path: &Path, reason: &str) -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "access a workspace path",
        format!("{}: {reason}", path.display()),
    )
    .with_recovery("select a regular file or directory directly inside the registered folder")
}

pub(super) fn conflict(path: &Path, reason: &str) -> AxError {
    AxError::failure(AxCode::VersionConflict, "verify a workspace file", format!("{}: {reason}", path.display()))
        .with_recovery("reopen the current file and review the difference before retrying; if it moved and identity could not be verified, explicitly relink the former record")
}
