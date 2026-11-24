// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

use kernel::{RESERVED_PREFIX, RunId};

use crate::error::StorageError;

use super::commit::{Checkpoint, git_err};

pub(crate) enum IndexOwner {
    City,

    Writer(PathBuf),
}

static HEAD_MOVES: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());
static RELEASED: Condvar = Condvar::new();

pub(crate) struct HeadGuard(PathBuf);

pub(crate) fn moving_head(path: &Path) -> Result<HeadGuard, StorageError> {
    let held = HEAD_MOVES.lock().unwrap_or_else(PoisonError::into_inner);
    let (mut held, _) = RELEASED
        .wait_timeout_while(held, Duration::from_secs(30), |held| held.contains(path))
        .unwrap_or_else(PoisonError::into_inner);
    if !held.insert(path.to_owned()) {
        return Err(StorageError::Checkpoint {
            op: "move HEAD",
            detail: format!("{} is still committing after 30 seconds", path.display()),
        });
    }
    Ok(HeadGuard(path.to_owned()))
}

impl Drop for HeadGuard {
    fn drop(&mut self) {
        HEAD_MOVES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.0);
        RELEASED.notify_all();
    }
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
    pub(super) fn head_lock(&self) -> &Path {
        match (&self.index, &self.reference) {
            (IndexOwner::Writer(path), Some(_)) => path,
            _ => self.repo.path(),
        }
    }

    pub fn open(city_root: &Path) -> Result<Checkpoint, StorageError> {
        let _initializing = moving_head(city_root)?;
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
        if cfg!(windows) && local.get_bool("core.longpaths") != Ok(true) {
            local
                .set_bool("core.longpaths", true)
                .map_err(git_err("enable long checkout paths"))?;
        }
        Ok(Checkpoint {
            repo,
            last: None,
            index: IndexOwner::City,
            reference: None,
            entropy_allow: kernel::EntropyAllowlist::default(),
        })
    }

    pub fn open_writer(root: &Path, writer: RunId) -> Result<Checkpoint, StorageError> {
        let repo =
            git2::Repository::open(root).map_err(git_err("open the private writer checkout"))?;
        Self::writer(repo, writers_dir(root), writer, None)
    }

    pub(crate) fn writer(
        repo: git2::Repository,
        dir: PathBuf,
        writer: RunId,
        reference: Option<String>,
    ) -> Result<Self, StorageError> {
        let own = dir.join(writer.to_string());
        let refused = |source| StorageError::Io {
            op: "open the writer index",
            path: own.clone(),
            source,
        };
        std::fs::create_dir_all(&dir).map_err(refused)?;
        let fresh = !own.try_exists().map_err(refused)?;
        let mut index = git2::Index::open(&own).map_err(git_err("open the writer index"))?;
        if fresh && let Some(commit) = Self::parent_at(&repo, reference.as_deref())? {
            let tree = commit.tree().map_err(git_err("read the writer baseline"))?;
            index
                .read_tree(&tree)
                .map_err(git_err("seed the writer index from its baseline"))?;
        }
        repo.set_index(&mut index)
            .map_err(git_err("give a writer its index"))?;
        Ok(Checkpoint {
            repo,
            last: None,
            index: IndexOwner::Writer(own),
            reference,
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
