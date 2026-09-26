// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A tree kept for its node between runs: whether somebody holds it,
//! and how it is taken back into use.

use super::super::lease::WorktreeLease;
use super::super::name::WorktreeName;
use super::super::weight::measure;
use super::Worktrees;
use crate::error::MemoryError;

/// Why a tree is locked: a run holds it.
const LEASE_REASON: &str = "held by a sprawling run";

/// What [`Worktrees::claim`] finds under a name.
pub(super) enum Standing {
    Held,
    Kept(git2::Worktree),
    Absent,
}

impl Worktrees {
    /// Where `name` stands: held under a lock, kept on disk for its
    /// node, or not there at all.
    ///
    /// A registration with no directory under it is what an interrupted
    /// placement or a person deleting the tree by hand leaves, and it is
    /// taken back here - the same reflex the index has about a cache it
    /// doubts - so that `E_WORKTREE_BUSY` keeps meaning that somebody is
    /// working.
    pub(super) fn standing(&self, name: &WorktreeName) -> Result<Standing, MemoryError> {
        let tree = match self.repo.find_worktree(name.as_str()) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Standing::Absent),
            Err(err) => {
                return Err(MemoryError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                });
            }
        };
        if !tree.path().exists() {
            self.forget(name)?;
            return Ok(Standing::Absent);
        }
        let lock = tree.is_locked().map_err(|err| MemoryError::Worktree {
            op: "read a worktree lock",
            detail: format!("{}: {err}", name.as_str()),
        })?;
        Ok(match lock {
            git2::WorktreeLockStatus::Locked(_) => Standing::Held,
            git2::WorktreeLockStatus::Unlocked => Standing::Kept(tree),
        })
    }

    /// Takes a kept tree back into use: the node's branch moved up to
    /// the trunk when the trunk already holds all of it, every tracked
    /// file forced to that branch head, every untracked file removed,
    /// then the lock. Git rewrites only the files that differ, so a node's second
    /// run costs what it left behind, not the size of the city.
    pub(super) fn reattach(
        &self,
        name: &WorktreeName,
        tree: &git2::Worktree,
    ) -> Result<WorktreeLease, MemoryError> {
        let refuse = |op: &'static str, err: git2::Error| MemoryError::Worktree {
            op,
            detail: format!("{}: {err}", name.as_str()),
        };
        self.follow_trunk(name)?;
        git2::Repository::open_from_worktree(tree)
            .map_err(|err| refuse("open a kept worktree", err))?
            .checkout_head(Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(true),
            ))
            .map_err(|err| refuse("reset a kept worktree", err))?;
        tree.lock(Some(LEASE_REASON))
            .map_err(|err| refuse("lock a worktree", err))?;
        Ok(WorktreeLease {
            name: name.clone(),
            path: tree.path().to_path_buf(),
            disk: measure(tree.path())?,
        })
    }

    /// Moves `name`'s branch to the trunk when the trunk descends from
    /// it. A branch with work the trunk lacks stays where it is: that
    /// work is still waiting for its merge.
    fn follow_trunk(&self, name: &WorktreeName) -> Result<(), MemoryError> {
        let refuse = |op: &'static str, err: git2::Error| MemoryError::Worktree {
            op,
            detail: format!("{}: {err}", name.as_str()),
        };
        let mut branch = self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
            .map_err(|err| refuse("find a node branch", err))?;
        let tip = branch
            .get()
            .peel_to_commit()
            .map_err(|err| refuse("read a node branch", err))?
            .id();
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|err| refuse("read the city trunk", err))?
            .id();
        let behind = tip != trunk
            && self
                .repo
                .graph_descendant_of(trunk, tip)
                .map_err(|err| refuse("judge a fast-forward", err))?;
        if behind {
            branch
                .get_mut()
                .set_target(trunk, "follow the trunk")
                .map_err(|err| refuse("move a node branch", err))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::tests::{city, name, owner};
    use crate::checkpoint::Checkpoint;
    use kernel::TimeMs;

    /// A node's next run finds its tree where it left it: releasing gives
    /// the lease back and keeps the files, so the next claim neither
    /// measures the city nor checks the whole tree out again.
    #[test]
    fn a_released_tree_stays_on_disk_for_the_nodes_next_run() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        let lease = trees.claim(&name("node-1")).unwrap();
        let path = lease.path().to_path_buf();
        let notes = path.join("lab").join("notes.md");
        let written = std::fs::metadata(&notes).unwrap().modified().unwrap();

        trees.release(lease).unwrap();
        assert!(notes.exists(), "a released tree keeps its files");
        assert_eq!(trees.live().unwrap(), vec![name("node-1")]);

        let again = trees.claim(&name("node-1")).unwrap();
        assert_eq!(again.path(), path.as_path());
        assert_eq!(
            std::fs::metadata(&notes).unwrap().modified().unwrap(),
            written,
            "a file the node's branch already holds is not written again"
        );
        assert!(dir.path().join("lab").join("notes.md").exists());
    }

    /// A kept tree whose node has nothing unmerged follows the trunk: a
    /// node that starts from where the city was when it last ran would do
    /// its next work on files somebody has since changed, and every merge
    /// of it would be refused as stale.
    #[test]
    fn a_kept_tree_with_nothing_unmerged_starts_from_the_trunk_as_it_now_stands() {
        let dir = tempfile::tempdir().unwrap();
        let trees = city(dir.path());
        trees
            .release(trees.claim(&name("node-1")).unwrap())
            .unwrap();
        std::fs::write(
            dir.path().join("lab").join("notes.md"),
            b"second
    ",
        )
        .unwrap();
        Checkpoint::open(dir.path())
            .unwrap()
            .land(TimeMs::new(2_000), &owner(), "checkpoint: lab")
            .unwrap();

        let again = trees.claim(&name("node-1")).unwrap();
        assert_eq!(
            std::fs::read_to_string(again.path().join("lab").join("notes.md")).unwrap(),
            "second
    "
        );
    }
}
