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
