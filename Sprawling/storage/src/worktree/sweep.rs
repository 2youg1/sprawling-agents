// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use crate::error::StorageError;

use super::name::WorktreeName;
use super::trees::{WORKTREE_DIR, Worktrees, open_city};

impl Worktrees {
    pub fn sweep_abandoned(
        city_root: &Path,
        held: &[WorktreeName],
    ) -> Result<Vec<WorktreeName>, StorageError> {
        let repo = match open_city(city_root) {
            Ok(repo) => repo,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(StorageError::Worktree {
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

    fn sweep_registered(&self, held: &[WorktreeName]) -> Result<Vec<WorktreeName>, StorageError> {
        let mut swept = Vec::new();
        for name in self.live()? {
            if held.contains(&name) || !self.made_here(&name)? {
                continue;
            }
            self.forget(name.as_str())?;
            self.drop_lease_branch(&name)?;
            swept.push(name);
        }
        Ok(swept)
    }

    fn made_here(&self, name: &WorktreeName) -> Result<bool, StorageError> {
        let tree =
            self.repo
                .find_worktree(name.as_str())
                .map_err(|err| StorageError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                })?;
        Ok(resolved(tree.path()) == resolved(&self.home.join(name.as_str())))
    }

    fn drop_lease_branch(&self, name: &WorktreeName) -> Result<(), StorageError> {
        let git_err = |op: &'static str| {
            move |err: git2::Error| StorageError::Worktree {
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

    pub(super) fn forget(&self, id: &str) -> Result<(), StorageError> {
        let admin = self.repo.commondir().join(WORKTREE_DIR).join(id);
        let tree = match self.repo.find_worktree(id) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),

            Err(_) if admin.is_dir() => {
                return std::fs::remove_dir_all(&admin).map_err(|source| StorageError::Worktree {
                    op: "take back a registration git will not read",
                    detail: format!("{id}: {source}"),
                });
            }
            Err(err) => {
                return Err(StorageError::Worktree {
                    op: "find a worktree",
                    detail: format!("{id}: {err}"),
                });
            }
        };
        let mut opts = git2::WorktreePruneOptions::new();

        opts.valid(true).locked(true).working_tree(true);
        tree.prune(Some(&mut opts))
            .map_err(|err| StorageError::Worktree {
                op: "prune a worktree",
                detail: format!("{id}: {err}"),
            })
    }

    pub(super) fn clear_unregistered(&self, id: &str) -> Result<(), StorageError> {
        let path = self.home.join(id);
        crate::alias::WriteTarget::within("place a worktree", &self.city_root, &path)?;
        match std::fs::remove_dir_all(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(StorageError::Io {
                op: "remove an abandoned worktree",
                path,
                source,
            }),
        }
    }

    fn sweep_unregistered(&self, held: &[WorktreeName]) -> Result<Vec<WorktreeName>, StorageError> {
        let io_err = |op: &'static str, path: &Path| {
            let path = path.to_path_buf();
            move |source| StorageError::Io { op, path, source }
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

    #[derive(Debug, PartialEq, Eq)]
    struct Left {
        registered: Vec<String>,
        dirs: Vec<String>,
        branches: Vec<String>,
        checkpoints: Vec<String>,
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
        let mut checkpoints: Vec<String> = repo
            .references_glob("refs/sprawling/runs/*")
            .unwrap()
            .map(|reference| reference.unwrap().name().unwrap().to_owned())
            .collect();
        checkpoints.sort();
        Left {
            registered,
            dirs,
            branches,
            checkpoints,
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

        drop(trees.claim(&name("run-1"), &[]).unwrap());
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
                    checkpoints: vec![],
                }
            )
        );
    }

    #[test]
    fn a_checkpoint_ref_survives_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        drop(trees.claim(&name("run-1"), &[]).unwrap());
        drop(trees);
        std::fs::write(dir.path().join("lab").join("notes.md"), b"second\n").unwrap();
        Checkpoint::open(dir.path())
            .unwrap()
            .wave_pre(&["lab".to_owned()], TimeMs::new(2_000), &owner())
            .unwrap();
        let checkpoints = left(dir.path()).checkpoints;
        assert_eq!(checkpoints.len(), 1, "the fixture checkpoints once");

        let swept = Worktrees::sweep_abandoned(dir.path(), &[]).unwrap();

        assert_eq!(
            (swept, left(dir.path()).checkpoints),
            (vec![name("run-1")], checkpoints)
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
        let live = trees.claim(&name("run-1"), &[]).unwrap();
        drop(trees.claim(&name("run-2"), &[]).unwrap());
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
                    checkpoints: vec![],
                },
                true
            )
        );
    }
}
