// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Component, Path, PathBuf, Prefix};

use super::Worktrees;
use crate::StorageError;
use crate::error::io_err;

const GITDIR_PREFIX: &str = "gitdir: ";

impl Worktrees {

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
            let Some(tree_git) = relocated(&gitdir, "", &from, city_root)? else {
                continue;
            };
            if let Some(moved) = relocated(&tree_git, GITDIR_PREFIX, &from, city_root)? {
                point(&tree_git, GITDIR_PREFIX, &moved)?;
            }

            let commondir = registration.join("commondir");
            if let Some(moved) = relocated(&commondir, "", &from, city_root)? {
                point(&commondir, "", &moved)?;
            }

            point(&gitdir, "", &tree_git)?;
            relinked = relinked.saturating_add(1);
        }
        Ok(relinked)
    }
}

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

fn relocated(
    file: &Path,
    prefix: &str,
    from: &[String],
    to: &Path,
) -> Result<Option<PathBuf>, StorageError> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(io_err("read a worktree link", file)(err)),
    };
    let named = text
        .trim_end()
        .strip_prefix(prefix)
        .unwrap_or(text.trim_end());
    Ok(under(named, from).map(|rest| to.join(rest)))
}

fn point(file: &Path, prefix: &str, path: &Path) -> Result<(), StorageError> {
    let spelled = path.to_string_lossy().replace('\\', "/");
    std::fs::write(file, format!("{prefix}{spelled}\n"))
        .map_err(io_err("write a worktree link", file))
}

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
