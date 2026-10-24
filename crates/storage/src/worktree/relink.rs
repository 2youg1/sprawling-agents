// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Pointing each tree's two links at the city's new folder after the
//! whole city moved (`crates/storage/spec/Worktree.lean` §8-9).
//!
//! git records a worktree by absolute path on both sides: the
//! repository's `worktrees/<name>/gitdir` names the tree's `.git` file,
//! that file names the registration back, and libgit2 writes the
//! registration's `commondir` absolute too. A city moved as one
//! folder keeps every byte of both, so both still name the old folder
//! and git no longer finds the tree. libgit2 has no repair, so the two
//! files are rewritten here, and only where they name a path under the
//! folder the city moved from.

use std::path::{Component, Path, PathBuf, Prefix};

use super::Worktrees;
use crate::StorageError;
use crate::error::io_err;

/// The line a tree's `.git` file opens with before the path it names.
const GITDIR_PREFIX: &str = "gitdir: ";

impl Worktrees {
    /// Re-points every registered tree of the repository at `city_root`
    /// from `moved_from` to `city_root`, and answers how many it
    /// re-pointed. A link that names a path outside `moved_from` is not
    /// the city's to move and is left as it is.
    ///
    /// # Errors
    /// A repository that cannot be read, and a link file that cannot be
    /// read or written.
    pub fn relink_moved(city_root: &Path, moved_from: &Path) -> Result<usize, StorageError> {
        let registrations = match std::fs::read_dir(city_root.join(".git").join("worktrees")) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => {
                return Err(io_err("list the city's worktrees", city_root)(err));
            }
        };
        let from = spellings(moved_from);
        let mut relinked: usize = 0;
        for entry in registrations {
            let registration = entry
                .map_err(io_err("list the city's worktrees", city_root))?
                .path();
            let gitdir = registration.join("gitdir");
            let Some(tree_git) = rewrite(&gitdir, "", &from, city_root)? else {
                continue;
            };
            rewrite(&tree_git, GITDIR_PREFIX, &from, city_root)?;
            // Absent or relative as plain git writes it; absolute as
            // libgit2 writes it, and then it names the old folder too.
            let commondir = registration.join("commondir");
            if commondir.exists() {
                rewrite(&commondir, "", &from, city_root)?;
            }
            relinked = relinked.saturating_add(1);
        }
        Ok(relinked)
    }
}

/// Every spelling git may have recorded the folder `from` under: as
/// given and as the disk resolves it, each with both slash spellings,
/// because git spells a Windows path with forward slashes and records
/// the resolved path. A city reached through a link (macOS puts
/// temporary folders under `/var`, a link to `/private/var`) or through
/// a Windows short name (`RUNNER~1`) is named by a path the caller never
/// spelled. The folder itself is gone after the move, so its parent is
/// what is resolved; a parent that cannot be resolved leaves only the
/// spelling as given, and a link under it is then left alone.
fn spellings(from: &Path) -> Vec<String> {
    let resolved = match (from.parent(), from.file_name()) {
        (Some(parent), Some(name)) => match std::fs::canonicalize(parent) {
            Ok(parent) => Some(without_verbatim_disk(&parent).join(name)),
            Err(_) => None,
        },
        (None, _) | (_, None) => None,
    };
    std::iter::once(from.to_path_buf())
        .chain(resolved)
        .flat_map(|path| {
            let given = path.to_string_lossy().into_owned();
            let forward = given.replace('\\', "/");
            [given, forward]
        })
        .map(|spelling| spelling.trim_end_matches(['/', '\\']).to_owned())
        .collect()
}

/// `\?\C:\city` respelled `C:\city`: Windows resolves a path into the
/// verbatim form, and git records the plain one.
fn without_verbatim_disk(path: &Path) -> PathBuf {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(letter) => {
                let mut plain = PathBuf::from(format!("{}:", char::from(letter)));
                plain.extend(components);
                plain
            }
            Prefix::Verbatim(_)
            | Prefix::VerbatimUNC(..)
            | Prefix::DeviceNS(_)
            | Prefix::UNC(..)
            | Prefix::Disk(_) => path.to_path_buf(),
        },
        Some(
            Component::RootDir | Component::CurDir | Component::ParentDir | Component::Normal(_),
        )
        | None => path.to_path_buf(),
    }
}

/// Rewrites the one path `file` holds after `prefix` when it lies under
/// one of the spellings in `from`, and answers the path it holds now;
/// `None` when it named a path outside `from` and was left alone.
fn rewrite(
    file: &Path,
    prefix: &str,
    from: &[String],
    to: &Path,
) -> Result<Option<PathBuf>, StorageError> {
    let text = std::fs::read_to_string(file).map_err(io_err("read a worktree link", file))?;
    let named = text
        .trim_end()
        .strip_prefix(prefix)
        .unwrap_or(text.trim_end());
    let Some(rest) = under(named, from) else {
        return Ok(None);
    };
    let moved = to.join(rest);
    // git writes these with forward slashes on every platform, and reads
    // either; keeping its spelling keeps the diff to the moved prefix.
    let spelled = moved.to_string_lossy().replace('\\', "/");
    std::fs::write(file, format!("{prefix}{spelled}\n"))
        .map_err(io_err("write a worktree link", file))?;
    Ok(Some(moved))
}

/// What follows one of the spellings in `from` in `named`.
fn under<'a>(named: &'a str, from: &[String]) -> Option<&'a str> {
    from.iter()
        .filter_map(|spelling| named.strip_prefix(spelling.as_str()))
        .find_map(|rest| rest.strip_prefix(['/', '\\']))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A city with one tree, moved as one folder, has a tree git can
    /// open again once the links are re-pointed.
    #[test]
    fn a_moved_city_finds_its_tree_again() {
        let dir = tempfile::tempdir().unwrap();
        let before = dir.path().join("beside").join("city");
        std::fs::create_dir_all(&before).unwrap();
        {
            let repo = git2::Repository::init(&before).unwrap();
            let signature = git2::Signature::now("city", "city@localhost").unwrap();
            let tree = repo.treebuilder(None).unwrap().write().unwrap();
            let tree = repo.find_tree(tree).unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "first", &tree, &[])
                .unwrap();
            let home = before.join(".sprawling").join("worktrees");
            std::fs::create_dir_all(&home).unwrap();
            repo.worktree("spare", &home.join("spare"), None).unwrap();
        }

        let after = dir.path().join("home").join("city");
        std::fs::create_dir_all(after.parent().unwrap()).unwrap();
        std::fs::rename(&before, &after).unwrap();
        let moved = git2::Repository::open(&after).unwrap();
        assert!(
            moved.find_worktree("spare").unwrap().validate().is_err(),
            "the links still name the old folder"
        );

        assert_eq!(Worktrees::relink_moved(&after, &before).unwrap(), 1);
        moved.find_worktree("spare").unwrap().validate().unwrap();
        git2::Repository::open(after.join(".sprawling").join("worktrees").join("spare")).unwrap();
        assert_eq!(
            Worktrees::relink_moved(&after, &before).unwrap(),
            0,
            "a second pass finds nothing under the old folder"
        );
    }
}
