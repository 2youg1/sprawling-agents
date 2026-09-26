// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The trees a crash left behind, swept when the city opens.
//!
//! A released tree stays on disk for the next claim of its name within
//! one serving; when the city opens, the ledger's writer lock means no
//! run holds any tree, so every tree the city made is left over from an
//! earlier serving, a crashed one included. What each leaves is three
//! things: git's registration, the directory under the reserved subtree,
//! and the branch git made for it. This module takes back each of them,
//! and only what the city made (memory-SPEC 8-9).

use std::path::{Path, PathBuf};

use crate::error::MemoryError;

use super::name::WorktreeName;
use super::trees::Worktrees;

impl Worktrees {
    /// Takes back every tree under `<city>/.sprawling/worktrees/` that
    /// no name in `held` holds, and answers the names taken, sorted.
    ///
    /// # Errors
    /// Propagates a repository that cannot be read, a registration git
    /// will not prune, a branch it will not delete, and a directory that
    /// cannot be removed.
    pub fn sweep_abandoned(
        city_root: &Path,
        held: &[WorktreeName],
    ) -> Result<Vec<WorktreeName>, MemoryError> {
        // A city that has never checkpointed has no repository, and so
        // no tree to leave behind.
        let repo = match git2::Repository::open(city_root) {
            Ok(repo) => repo,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(MemoryError::Worktree {
                    op: "open the city repository",
                    detail: format!("{}: {err}", city_root.display()),
                });
            }
        };
        let trees = Worktrees::over(repo, city_root);
        let mut swept = trees.sweep_registered(held)?;
        swept.extend(trees.sweep_unregistered(held)?);
        swept.sort();
        swept.dedup();
        Ok(swept)
    }

    /// Unregisters every tree of the city's own that nobody holds, and
    /// the lease branch that goes with it.
    fn sweep_registered(&self, held: &[WorktreeName]) -> Result<Vec<WorktreeName>, MemoryError> {
        let mut swept = Vec::new();
        for name in self.live()? {
            if held.contains(&name) || !self.made_here(&name)? {
                continue;
            }
            self.forget(&name)?;
            self.drop_lease_branch(&name)?;
            swept.push(name);
        }
        Ok(swept)
    }

    /// Whether git's registration of `name` is exactly `home/<name>` of
    /// this city. The common git dir lists trees of every city and
    /// linked checkout that shares it, so a matching path suffix proves
    /// nothing; both sides are resolved on disk, because the city root
    /// this process was given may be spelled differently from the one
    /// the tree was claimed under.
    fn made_here(&self, name: &WorktreeName) -> Result<bool, MemoryError> {
        let tree = self
            .repo
            .find_worktree(name.as_str())
            .map_err(|err| MemoryError::Worktree {
                op: "find a worktree",
                detail: format!("{}: {err}", name.as_str()),
            })?;
        Ok(resolved(tree.path()) == resolved(&self.home.join(name.as_str())))
    }

    /// Deletes the branch git made for the tree `name`, unless its tip
    /// carries a commit the trunk does not have - that is a run's landed
    /// offer, and the pull request names it - or the person has it
    /// checked out, which makes it theirs.
    fn drop_lease_branch(&self, name: &WorktreeName) -> Result<(), MemoryError> {
        let git_err = |op: &'static str| {
            move |err: git2::Error| MemoryError::Worktree {
                op,
                detail: format!("{}: {err}", name.as_str()),
            }
        };
        let mut branch = match self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
        {
            Ok(branch) => branch,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
            Err(err) => return Err(git_err("find a lease branch")(err)),
        };
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(git_err("read the trunk"))?
            .id();
        let tip = branch
            .get()
            .peel_to_commit()
            .map_err(git_err("read a lease branch"))?
            .id();
        let reached = tip == trunk
            || self
                .repo
                .graph_descendant_of(trunk, tip)
                .map_err(git_err("judge a lease branch"))?;
        if reached && !branch.is_head() {
            branch.delete().map_err(git_err("delete a lease branch"))?;
        }
        Ok(())
    }

    /// Removes every directory under the city's tree home that git has
    /// no registration for and nobody holds: a claim or a release the
    /// process died inside. The whole subtree is the city's machinery,
    /// so nothing in it is the person's.
    fn sweep_unregistered(&self, held: &[WorktreeName]) -> Result<Vec<WorktreeName>, MemoryError> {
        let io_err = |op: &'static str, path: &Path| {
            let path = path.to_path_buf();
            move |source| MemoryError::Io { op, path, source }
        };
        let entries = match std::fs::read_dir(&self.home) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(io_err("list the worktree home", &self.home)(err)),
        };
        let registered = self.live()?;
        let mut swept = Vec::new();
        for entry in entries {
            let entry = entry.map_err(io_err("list the worktree home", &self.home))?;
            // A name this module could not have written is not one it
            // may remove.
            let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|raw| WorktreeName::parse(raw).ok())
            else {
                continue;
            };
            if held.contains(&name) || registered.contains(&name) {
                continue;
            }
            let path = entry.path();
            std::fs::remove_dir_all(&path)
                .map_err(io_err("remove an abandoned worktree", &path))?;
            swept.push(name);
        }
        Ok(swept)
    }
}

/// `path` as the disk spells it: the whole path when it exists, else its
/// parent resolved and the last component kept, else `path` unchanged. A
/// crash may have removed the tree's own directory while its home stays.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .or_else(|_| match (path.parent(), path.file_name()) {
            (Some(parent), Some(leaf)) => std::fs::canonicalize(parent).map(|dir| dir.join(leaf)),
            (None, _) | (_, None) => Ok(path.to_path_buf()),
        })
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::path::Path;

    use kernel::TimeMs;

    use super::super::trees::tests::{city, owner};
    use super::*;
    use crate::checkpoint::Checkpoint;

    /// What a sweep can touch, read straight from git and the disk.
    #[derive(Debug, PartialEq, Eq)]
    struct Left {
        registered: Vec<String>,
        dirs: Vec<String>,
        branches: Vec<String>,
        fences: Vec<String>,
    }

    fn left(city_root: &Path) -> Left {
        let repo = git2::Repository::open(city_root).unwrap();
        let mut registered: Vec<String> = repo
            .worktrees()
            .unwrap()
            .iter()
            .flatten()
            .flatten()
            .map(str::to_owned)
            .collect();
        registered.sort();
        let home = city_root.join(kernel::RESERVED_PREFIX).join("worktrees");
        let mut dirs: Vec<String> = std::fs::read_dir(&home)
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                    .collect()
            })
            .unwrap_or_default();
        dirs.sort();
        let mut branches: Vec<String> = repo
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .map(|branch| branch.unwrap().0.name().unwrap().unwrap().to_owned())
            .collect();
        branches.sort();
        let mut fences: Vec<String> = repo
            .references_glob("refs/sprawling/runs/*")
            .unwrap()
            .map(|reference| reference.unwrap().name().unwrap().to_owned())
            .collect();
        fences.sort();
        Left {
            registered,
            dirs,
            branches,
            fences,
        }
    }

    fn name(raw: &str) -> WorktreeName {
        WorktreeName::parse(raw).unwrap()
    }

    fn trunk(city_root: &Path) -> String {
        let repo = git2::Repository::open(city_root).unwrap();
        repo.head().unwrap().shorthand().unwrap().to_owned()
    }

    #[test]
    fn a_crash_left_lease_is_swept_when_the_city_opens() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        // The process died holding the lease: nothing released it.
        drop(trees.claim(&name("run-1")).unwrap());
        drop(trees);

        let swept = Worktrees::sweep_abandoned(dir.path(), &[]).unwrap();

        assert_eq!(
            (swept, left(dir.path())),
            (
                vec![name("run-1")],
                Left {
                    registered: vec![],
                    dirs: vec![],
                    branches: vec![trunk(dir.path())],
                    fences: vec![],
                }
            )
        );
    }

    #[test]
    fn a_fence_ref_survives_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        drop(trees.claim(&name("run-1")).unwrap());
        drop(trees);
        std::fs::write(dir.path().join("lab").join("notes.md"), b"second\n").unwrap();
        Checkpoint::open(dir.path())
            .unwrap()
            .wave_pre(&["lab".to_owned()], TimeMs::new(2_000), &owner())
            .unwrap();
        let fences = left(dir.path()).fences;
        assert_eq!(fences.len(), 1, "the fixture fences once");

        let swept = Worktrees::sweep_abandoned(dir.path(), &[]).unwrap();

        assert_eq!(
            (swept, left(dir.path()).fences),
            (vec![name("run-1")], fences)
        );
    }

    #[test]
    fn a_tree_under_another_citys_home_survives_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        drop(city(dir.path()));
        let foreign = elsewhere
            .path()
            .join(kernel::RESERVED_PREFIX)
            .join("worktrees")
            .join("run-9");
        std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
        let repo = git2::Repository::open(dir.path()).unwrap();
        repo.worktree("run-9", &foreign, None).unwrap();

        let swept = Worktrees::sweep_abandoned(dir.path(), &[]).unwrap();

        assert_eq!(
            (swept, left(dir.path()).registered, foreign.exists()),
            (vec![], vec!["run-9".to_owned()], true)
        );
    }

    #[test]
    fn a_live_runs_tree_and_the_persons_own_tree_survive_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let live = trees.claim(&name("run-1")).unwrap();
        drop(trees.claim(&name("run-2")).unwrap());
        drop(trees);
        let repo = git2::Repository::open(dir.path()).unwrap();
        repo.worktree("mine", &elsewhere.path().join("mine"), None)
            .unwrap();

        let swept = Worktrees::sweep_abandoned(dir.path(), &[name("run-1")]).unwrap();

        let mut branches = vec!["mine".to_owned(), "run-1".to_owned(), trunk(dir.path())];
        branches.sort();
        assert_eq!(
            (swept, left(dir.path()), live.path().join("lab").exists()),
            (
                vec![name("run-2")],
                Left {
                    registered: vec!["mine".to_owned(), "run-1".to_owned()],
                    dirs: vec!["run-1".to_owned()],
                    branches,
                    fences: vec![],
                },
                true
            )
        );
    }
}
