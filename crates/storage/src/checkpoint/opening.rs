// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a handle onto the city repository is opened: on the repository's
//! own index, or on one writer's private index.
//!
//! Several runs checkpoint one city at once. What they share is the object
//! store, whose objects are named by their content, and references named by
//! the commit they hold; neither can be written two different ways. The index
//! is the one file two writers would disagree about, so each writer stages
//! into its own, under the reserved subtree where no checkpoint stages it.
//!
//! Specified by `crates/storage/spec/Checkpoint/Concurrent.lean` §8-39.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use kernel::{RESERVED_PREFIX, RunId};

use crate::error::StorageError;

use super::commit::{Checkpoint, git_err};

/// Which index a handle stages into.
pub(crate) enum IndexOwner {
    /// The repository's own index, the one a person's `git add` writes.
    City,
    /// One writer's private index file, deleted when the writer closes.
    Writer(PathBuf),
}

/// The steps that move HEAD, a base (`ensure_base`, `base_checkpoint`) and
/// `land`, take this in one
/// process, around their staging as well as their commit: both stage an index
/// another writer may stage at the same moment (the city's own, for a base),
/// and on Windows two handles rewriting one index file refuse each other's
/// rename. Across processes HEAD's compare-and-swap is what holds; on a
/// network drive the reference lock file is not exclusive, and no safe
/// interface tells the two apart (storage D26). Wave checkpoints never take it,
/// and `ensure_base` takes it only while the city has no commit.
static HEAD_MOVES: Mutex<()> = Mutex::new(());

/// Takes the in-process lock over moving HEAD.
///
/// # Errors
/// A lock that a thread which panicked while moving HEAD left behind.
pub(crate) fn moving_head() -> Result<MutexGuard<'static, ()>, StorageError> {
    HEAD_MOVES.lock().map_err(|_| StorageError::Checkpoint {
        op: "move HEAD",
        detail: "a thread failed while it was moving HEAD".to_owned(),
    })
}

/// What a commit does to HEAD. A wave checkpoint, and a base before its pack
/// is on disk, leave it: a wave checkpoint is filed under its own reference so
/// a person's `git log` does not grow a line per tool wave. A landing moves
/// it, because offered work has to be on a branch; a base moves it by
/// creating the branch afterwards (`checkpoint::base`).
pub(crate) enum HeadMove {
    Leave,
    /// From where HEAD stands when the commit is made: a landing.
    Advance,
}

/// What a refused commit says. HEAD moved since it was read, the branch a
/// base would create already exists, or its lock file is held (on Windows,
/// also a rename onto a file someone has open): the
/// compare-and-swap refused, and a silent redo would treat the other writer's
/// work as this commit's to undo (storage §8-39).
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

/// Where the private indexes of the writers in `root` live.
fn writers_dir(root: &Path) -> PathBuf {
    root.join(RESERVED_PREFIX).join("index")
}

impl Checkpoint {
    /// Opens the city repository, initialising one when absent. The
    /// genesis commit is the first wave's, not this call's: an empty
    /// repository is a valid state, and inventing history here would
    /// make the first checkpoint unattributable.
    ///
    /// # Errors
    /// A repository that neither opens nor initialises, and a
    /// configuration that cannot be read or written.
    pub fn open(city_root: &Path) -> Result<Checkpoint, StorageError> {
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(_) => git2::Repository::init(city_root).map_err(git_err("init repository"))?,
        };
        // The city's files round-trip byte for byte, whatever this
        // machine's git is configured to do to other people's
        // repositories. A checkout that rewrote line endings would make
        // a file disagree with the hash the ledger holds for it, and the
        // disagreement would look like corruption rather than like a
        // setting. Checked on every open, because the setting is a property
        // of this repository rather than of the moment it was created; written
        // only when it differs, because a write takes the config lock and
        // two lanes opening at once would race on it.
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
        })
    }

    /// A handle that stages into `writer`'s own index under
    /// `<root>/.sprawling/index/<run>`, seeded from the repository's index
    /// so the first checkpoint still skips unchanged files by their stat.
    ///
    /// # Errors
    /// No repository at `root` (writers never create one), and an index file
    /// that cannot be seeded or opened.
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
        })
    }

    /// Deletes this writer's index file once its run has landed. A handle on
    /// the repository's own index deletes nothing.
    ///
    /// # Errors
    /// A file the file system will not delete.
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

    /// Deletes every private index under `root`, and answers how many. Called
    /// when the city opens, when no run is live, so every one of them is what
    /// a crash left behind.
    ///
    /// # Errors
    /// A directory that cannot be listed and a file that will not delete.
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
