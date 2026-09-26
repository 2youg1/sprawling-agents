// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The trees a crash left behind, swept when the city opens.
//!
//! A tree goes back only through [`Worktrees::release`], and its name is
//! a run id that never comes back, so [`Worktrees::claim`]'s own repair
//! never meets the tree of a run whose process died. What such a run
//! leaves is three things: git's registration, the directory under the
//! reserved subtree, and the branch git made for it. This module takes
//! back each of them, and only what the city made (memory-SPEC 8-9).

use std::path::Path;

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
        drop((city_root, held));
        Ok(Vec::new())
    }
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
