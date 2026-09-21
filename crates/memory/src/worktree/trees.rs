// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Worktrees: claim, merge, release.

use std::path::{Path, PathBuf};

use kernel::{ByteLen, consts_policy::WORKTREE_MAX_BYTES};

use crate::error::MemoryError;

use super::landing::{Landing, PlannedMerge};
use super::lease::WorktreeLease;
use super::name::WorktreeName;
use super::weight::measure;

/// Where the trees live: inside the reserved subtree, because they are
/// the city's own machinery rather than anybody's writable space. What a
/// run may write is judged against the tree it works in.
const WORKTREE_DIR: &str = "worktrees";

/// The city's trees.
pub struct Worktrees {
    repo: git2::Repository,
    home: PathBuf,
    ceiling: ByteLen,
}

// Hand-written because a git repository handle has no Debug: what a
// reader of a failure needs is where the trees go and what they may
// cost, not the handle.
impl std::fmt::Debug for Worktrees {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worktrees")
            .field("home", &self.home)
            .field("ceiling", &self.ceiling)
            .finish()
    }
}

impl Worktrees {
    /// Opens the city's repository as the source every tree branches
    /// from.
    ///
    /// # Errors
    /// Refuses a city with no repository. Initialising one here would
    /// make two modules able to create the city's history.
    pub fn open(city_root: &Path) -> Result<Worktrees, MemoryError> {
        let repo = git2::Repository::open(city_root).map_err(|err| MemoryError::Worktree {
            op: "open the city repository",
            detail: format!("{}: {err}", city_root.display()),
        })?;
        Ok(Worktrees {
            repo,
            home: city_root.join(kernel::RESERVED_PREFIX).join(WORKTREE_DIR),
            ceiling: ByteLen::new(WORKTREE_MAX_BYTES),
        })
    }

    /// Opens a tree for one node.
    ///
    /// # Errors
    /// Refuses a name already in use, a city whose working tree exceeds
    /// the ceiling, and a repository with no commit to branch from.
    pub fn claim(&self, name: &WorktreeName) -> Result<WorktreeLease, MemoryError> {
        if self.live()?.contains(name) && !self.reclaim_abandoned(name)? {
            return Err(MemoryError::WorktreeBusy {
                name: name.as_str().to_owned(),
                detail: "another node holds this tree".to_owned(),
            });
        }
        let source = self.repo.workdir().ok_or_else(|| MemoryError::Worktree {
            op: "find the city working tree",
            detail: "the repository is bare".to_owned(),
        })?;
        let size = measure(source)?;
        if size.get() > self.ceiling.get() {
            return Err(MemoryError::WorktreeBusy {
                name: name.as_str().to_owned(),
                detail: format!(
                    "the city working tree is {} bytes and the ceiling is {}",
                    size.get(),
                    self.ceiling.get()
                ),
            });
        }
        if self.repo.head().is_err() {
            return Err(MemoryError::Worktree {
                op: "branch a worktree",
                detail: "the city has no checkpoint yet, and a tree branches from a commit"
                    .to_owned(),
            });
        }
        std::fs::create_dir_all(&self.home).map_err(|source| MemoryError::Io {
            op: "create the worktree home",
            path: self.home.clone(),
            source,
        })?;
        let path = self.home.join(name.as_str());
        // A node that has held a tree before still has its branch: the
        // tree is a materialization, the branch is the line of work.
        // Reattaching is what makes releasing a tree cheap enough to do
        // between sessions.
        let branch = self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
            .ok();
        let mut opts = git2::WorktreeAddOptions::new();
        if let Some(branch) = branch.as_ref() {
            opts.reference(Some(branch.get()));
        }
        self.repo
            .worktree(name.as_str(), &path, Some(&opts))
            .map_err(|err| MemoryError::Worktree {
                op: "add a worktree",
                detail: format!("{}: {err}", name.as_str()),
            })?;
        Ok(WorktreeLease {
            name: name.clone(),
            path: path.clone(),
            disk: measure(&path)?,
        })
    }

    /// Decides a merge without making it.
    ///
    /// Fast-forward only. A node whose trunk moved underneath it does
    /// not get its work merged on top of somebody else's by a machine:
    /// it rebuilds on the trunk as it now stands and is verified again.
    /// This is the same stance the draft desk takes about a room that
    /// moved, and for the same reason - the party who knows whether the
    /// work is still right is the one who did it.
    ///
    /// Every refusal happens here, before anything moves, and the commit
    /// the trunk will point at is already known - so a caller can write
    /// the line that announces the merge before the merge exists, and
    /// still never announce one that was going to be refused.
    ///
    /// # Errors
    /// Refuses an unknown branch, a repository with no commit, and a
    /// merge that is not a fast-forward.
    pub fn plan_merge(&self, name: &WorktreeName) -> Result<PlannedMerge<'_>, MemoryError> {
        let refuse = |op: &'static str, detail: String| MemoryError::Worktree { op, detail };
        let branch = self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
            .map_err(|err| refuse("find a node branch", format!("{}: {err}", name.as_str())))?;
        let theirs = branch
            .get()
            .peel_to_commit()
            .map_err(|err| refuse("read a node branch", err.to_string()))?;
        let head = self
            .repo
            .head()
            .map_err(|err| refuse("read the city trunk", err.to_string()))?;
        let ours = head
            .peel_to_commit()
            .map_err(|err| refuse("read the city trunk", err.to_string()))?;
        // A git failure here is not an answer to the ancestry question.
        // Reading it as "not a descendant" would report a stale trunk
        // to somebody whose trunk is fine, and send them to rebuild
        // work that needed no rebuilding.
        let descends = theirs.id() == ours.id()
            || self
                .repo
                .graph_descendant_of(theirs.id(), ours.id())
                .map_err(|err| refuse("judge a fast-forward", err.to_string()))?;
        if !descends {
            return Err(MemoryError::MergeStale {
                name: name.as_str().to_owned(),
                detail: format!("the trunk moved to {} after this node branched", ours.id()),
            });
        }
        Ok(PlannedMerge {
            trees: self,
            target: theirs.id(),
        })
    }

    /// Writes the merge commit [`Worktrees::plan_merge`] settled on: two
    /// parents, the node's tree, and the merging run's trailers.
    pub(super) fn land_merge(
        &self,
        target: git2::Oid,
        landing: &Landing<'_>,
    ) -> Result<(), MemoryError> {
        let refuse = |op: &'static str, detail: String| MemoryError::Worktree { op, detail };
        let head = self
            .repo
            .head()
            .map_err(|err| refuse("read the city trunk", err.to_string()))?;
        let ours = head
            .peel_to_commit()
            .map_err(|err| refuse("read the city trunk", err.to_string()))?;
        let theirs = self
            .repo
            .find_commit(target)
            .map_err(|err| refuse("read the node commit", err.to_string()))?;
        // The node's tree: the trunk is an ancestor by the fast-forward
        // judgement, so it has nothing the node does not carry.
        let tree = theirs
            .tree()
            .map_err(|err| refuse("read the node tree", err.to_string()))?;
        let seconds = i64::try_from(landing.t.value().saturating_div(1000)).unwrap_or(0);
        let when = git2::Time::new(seconds, 0);
        let email = landing.of.email();
        let signature = git2::Signature::new(landing.of.actor().as_str(), &email, &when)
            .map_err(|err| refuse("build the merge signature", err.to_string()))?;
        let message = format!(
            "{}\n\n{}{}",
            landing.subject,
            landing.of.trailers(),
            self.reviewer(landing.reviewed_by_person)
        );
        self.repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                &message,
                &tree,
                &[&ours, &theirs],
            )
            .map_err(|err| refuse("commit the merge", err.to_string()))?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        self.repo
            .checkout_head(Some(&mut checkout))
            .map_err(|err| refuse("check out the merged trunk", err.to_string()))?;
        Ok(())
    }

    /// The `Reviewed-by:` line, or nothing at all. Both halves have to
    /// hold: the caller says a person looked, and this machine's git
    /// config says who that person is. A name the city made up would be
    /// a false attribution in somebody's own repository.
    fn reviewer(&self, reviewed_by_person: bool) -> String {
        if !reviewed_by_person {
            return String::new();
        }
        let Ok(config) = self.repo.config() else {
            return String::new();
        };
        match (
            config.get_string("user.name"),
            config.get_string("user.email"),
        ) {
            (Ok(name), Ok(email)) => format!("Reviewed-by: {name} <{email}>\n"),
            _ => String::new(),
        }
    }

    /// Gives a tree back: the repository forgets it, then whatever is
    /// left of its files goes.
    ///
    /// That order, because the registration is what makes the name
    /// unusable. Removing the directory first and then failing left the
    /// name registered with nothing behind it, and the only thing
    /// [`Worktrees::claim`] could say about it was that somebody else
    /// held it — a tree nobody held and nobody could take.
    ///
    /// # Errors
    /// Propagates a repository that refuses to prune and a directory
    /// that cannot be removed.
    pub fn release(&self, lease: WorktreeLease) -> Result<(), MemoryError> {
        self.forget(lease.name())?;
        if lease.path().exists() {
            std::fs::remove_dir_all(lease.path()).map_err(|source| MemoryError::Io {
                op: "remove a worktree",
                path: lease.path().to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }

    /// Unregisters `name`, taking its files with it where git will.
    ///
    /// A repository that has already forgotten the tree is the end
    /// state this asks for, so it is not a failure.
    fn forget(&self, name: &WorktreeName) -> Result<(), MemoryError> {
        let tree = match self.repo.find_worktree(name.as_str()) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
            Err(err) => {
                return Err(MemoryError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                });
            }
        };
        let mut opts = git2::WorktreePruneOptions::new();
        opts.valid(true).working_tree(true);
        tree.prune(Some(&mut opts))
            .map_err(|err| MemoryError::Worktree {
                op: "prune a worktree",
                detail: format!("{}: {err}", name.as_str()),
            })
    }

    /// Takes back a registration whose directory is gone, and says
    /// whether the name is free again.
    ///
    /// A release interrupted between the two halves, or a directory a
    /// person deleted by hand, leaves exactly this: a name git still
    /// lists with no tree under it. Rebuilding it is the same reflex
    /// the index has about a cache it doubts, and it is what keeps
    /// `E_WORKTREE_BUSY` meaning that somebody is working.
    fn reclaim_abandoned(&self, name: &WorktreeName) -> Result<bool, MemoryError> {
        let registered =
            self.repo
                .find_worktree(name.as_str())
                .map_err(|err| MemoryError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                })?;
        if registered.path().exists() {
            return Ok(false);
        }
        self.forget(name)?;
        Ok(true)
    }

    /// Every tree the repository knows about, sorted.
    ///
    /// # Errors
    /// Propagates a repository that cannot list its worktrees.
    pub fn live(&self) -> Result<Vec<WorktreeName>, MemoryError> {
        let names = self.repo.worktrees().map_err(|err| MemoryError::Worktree {
            op: "list worktrees",
            detail: err.to_string(),
        })?;
        let mut out = Vec::new();
        // A name git cannot render as UTF-8 was not written by this
        // module, and it is not a tree this city can address.
        for name in names.iter().flatten().flatten() {
            out.push(WorktreeName::parse(name)?);
        }
        out.sort();
        Ok(out)
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
pub(crate) mod tests;
