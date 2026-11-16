// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use kernel::{RESERVED_PREFIX, RunId};

use crate::error::StorageError;

use super::commit::{Checkpoint, git_err};

pub(crate) enum IndexOwner {

    City,

    Writer(PathBuf),
}

static HEAD_MOVES: Mutex<()> = Mutex::new(());

pub(crate) fn moving_head() -> Result<MutexGuard<'static, ()>, StorageError> {
    HEAD_MOVES.lock().map_err(|_| StorageError::Checkpoint {
        op: "move HEAD",
        detail: "a thread failed while it was moving HEAD".to_owned(),
    })
}

pub(crate) enum HeadMove {
    Leave,

    Advance,
}

pub(crate) fn commit_refused(update: Option<&str>, err: git2::Error) -> StorageError {
    let lost = update.is_some()
        && matches!(
            err.code(),
            git2::ErrorCode::Modified | git2::ErrorCode::Locked | git2::ErrorCode::Exists
        );
    if lost {
        StorageError::Checkpoint {
            op: "move HEAD",
            detail: format!("another writer moved or holds HEAD: {}", err.message()),
        }
    } else {
        git_err("commit checkpoint")(err)
    }
}

fn writers_dir(root: &Path) -> PathBuf {
    root.join(RESERVED_PREFIX).join("index")
}

impl Checkpoint {

    pub fn open(city_root: &Path) -> Result<Checkpoint, StorageError> {
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(err) if err.code() == git2::ErrorCode::NotFound => {
                git2::Repository::init(city_root).map_err(git_err("init repository"))?
            }
            Err(err) => return Err(git_err("open the city repository")(err)),
        };

        let mut local = repo
            .config()
            .and_then(|config| config.open_level(git2::ConfigLevel::Local))
            .map_err(git_err("read the repository's configuration"))?;
        let pinned = match local.get_bool("core.autocrlf") {
            Ok(value) => !value,
            Err(err) if err.code() == git2::ErrorCode::NotFound => false,
            Err(err) => return Err(git_err("read the repository's line endings")(err)),
        };
        if !pinned {
            local
                .set_bool("core.autocrlf", false)
                .map_err(git_err("pin the repository's line endings"))?;
        }
        Ok(Checkpoint {
            repo,
            last: None,
            index: IndexOwner::City,
            entropy_allow: kernel::EntropyAllowlist::default(),
        })
    }

    pub fn open_writer(root: &Path, writer: RunId) -> Result<Checkpoint, StorageError> {
        let repo = git2::Repository::open(root)
            .map_err(git_err("open the city repository for a writer"))?;
        let dir = writers_dir(root);
        let own = dir.join(writer.to_string());
        let seed = repo.path().join("index");
        let refused = |detail: String| StorageError::Checkpoint {
            op: "seed a writer's index",
            detail,
        };
        let present = |path: &Path| {
            path.try_exists()
                .map_err(|err| refused(format!("{}: {err}", path.display())))
        };
        std::fs::create_dir_all(&dir)
            .map_err(|err| refused(format!("{}: {err}", dir.display())))?;
        if !present(&own)? && present(&seed)? {
            std::fs::copy(&seed, &own)
                .map_err(|err| refused(format!("{}: {err}", own.display())))?;
        }
        let mut index = git2::Index::open(&own).map_err(git_err("open a writer's index"))?;
        repo.set_index(&mut index)
            .map_err(git_err("give a writer its index"))?;
        Ok(Checkpoint {
            repo,
            last: None,
            index: IndexOwner::Writer(own),
            entropy_allow: kernel::EntropyAllowlist::default(),
        })
    }

    pub fn with_entropy_allowlist(mut self, allowed: kernel::EntropyAllowlist) -> Checkpoint {
        self.entropy_allow = allowed;
        self
    }

    pub fn close_writer(self) -> Result<(), StorageError> {
        let Checkpoint { repo, index, .. } = self;
        drop(repo);
        match index {
            IndexOwner::City => Ok(()),
            IndexOwner::Writer(own) => match std::fs::remove_file(&own) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(StorageError::Checkpoint {
                    op: "close a writer's index",
                    detail: format!("{}: {err}", own.display()),
                }),
            },
        }
    }

    pub fn sweep_writers(root: &Path) -> Result<usize, StorageError> {
        let dir = writers_dir(root);
        let refused = |detail: String| StorageError::Checkpoint {
            op: "sweep the writers' indexes",
            detail,
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => return Err(refused(format!("{}: {err}", dir.display()))),
        };
        let mut swept: usize = 0;
        for entry in entries {
            let path = entry
                .map_err(|err| refused(format!("{}: {err}", dir.display())))?
                .path();
            std::fs::remove_file(&path)
                .map_err(|err| refused(format!("{}: {err}", path.display())))?;
            swept = swept.saturating_add(1);
        }
        Ok(swept)
    }
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
