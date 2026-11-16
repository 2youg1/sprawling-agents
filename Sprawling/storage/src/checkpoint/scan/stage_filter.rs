// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::StorageError;

pub(super) fn workdir(repo: &git2::Repository) -> Result<&std::path::Path, StorageError> {
    repo.workdir().ok_or_else(|| StorageError::Checkpoint {
        op: "stage a wave",
        detail: "the city repository is bare".to_owned(),
    })
}

fn is_half_a_write(relative: &std::path::Path) -> bool {
    relative.file_name().is_some_and(|name| {
        kernel::layout::is_document_staging_name(name)
            || crate::bundle::landing::is_staging_name(name)
    })
}

pub(super) struct StageFilter {
    root: std::path::PathBuf,
    aliases: Vec<(String, crate::alias::AliasKind)>,
    nested: Vec<std::path::PathBuf>,
    fault: Option<StorageError>,
}

impl StageFilter {
    pub(super) fn new(root: &std::path::Path) -> StageFilter {
        StageFilter {
            root: root.to_path_buf(),
            aliases: Vec::new(),
            nested: Vec::new(),
            fault: None,
        }
    }

    pub(super) fn admit(&mut self, relative: &std::path::Path) -> i32 {
        let verdict = self.admit_entry(relative);
        if verdict != 0 {
            return verdict;
        }
        match self.nested_repository(relative) {
            Ok(true) => 1,
            Ok(false) => 0,
            Err(err) => {
                self.fault = Some(err);
                1
            }
        }
    }

    pub(super) fn admit_directory(&mut self, relative: &std::path::Path) -> i32 {
        self.admit_entry(relative)
    }

    fn admit_entry(&mut self, relative: &std::path::Path) -> i32 {
        if crate::sessions::is_session_projection(relative)
            || !crate::reserved::outside_reserved(relative)
            || is_half_a_write(relative)
        {
            return 1;
        }
        match crate::alias::kind_at(&self.root.join(relative)) {
            Ok(None) => 0,
            Ok(Some(kind)) => {
                self.aliases.push((relative.display().to_string(), kind));
                1
            }
            Err(err) => {
                self.fault = Some(err);
                1
            }
        }
    }

    fn nested_repository(&mut self, relative: &std::path::Path) -> Result<bool, StorageError> {
        match std::fs::symlink_metadata(self.root.join(relative).join(".git")) {
            Ok(_) => {
                self.nested.push(relative.to_path_buf());
                Ok(true)
            }

            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                Ok(false)
            }
            Err(err) => Err(StorageError::Checkpoint {
                op: "inspect a nested repository",
                detail: format!("{}: {err}", relative.display()),
            }),
        }
    }

    pub(super) fn nested(&self) -> &[std::path::PathBuf] {
        &self.nested
    }

    pub(super) fn refused(&mut self) -> Result<(), StorageError> {
        if let Some(err) = self.fault.take() {
            return Err(err);
        }
        self.aliases.sort_by(|left, right| left.0.cmp(&right.0));
        match self.aliases.first() {
            Some((path, kind)) => Err(crate::alias::refused(
                "stage a wave",
                std::path::Path::new(path),
                *kind,
            )),
            None => Ok(()),
        }
    }
}
