// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Worktrees: claim, merge, release.

use std::path::{Path, PathBuf};

use kernel::{ByteLen, consts_policy::WORKTREE_MAX_BYTES};

use crate::error::StorageError;

use super::landing::{CheckoutRun, Landing, PlannedMerge, check_out};
use super::lease::{FileWork, WorktreeLease};
use super::name::WorktreeName;
use super::weight::{Weight, Written, measure, written};
use kept::Standing;

mod kept;
mod stock;

/// Where the trees live: inside the reserved subtree, because they are
/// the city's own machinery rather than anybody's writable space. What a
/// run may write is judged against the tree it works in.
pub(super) const WORKTREE_DIR: &str = "worktrees";

/// The city's trees.
pub struct Worktrees {
    pub(super) repo: git2::Repository,
    pub(super) home: PathBuf,
    /// Where the person placed the city; the alias check stops here.
    pub(super) city_root: PathBuf,
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
    pub fn open(city_root: &Path) -> Result<Worktrees, StorageError> {
        let repo = open_city(city_root).map_err(|err| StorageError::Worktree {
            op: "open the city repository",
            detail: format!("{}: {err}", city_root.display()),
        })?;
        Ok(Worktrees::over(repo, city_root))
    }

    /// The trees of a repository already open at `city_root`.
    pub(super) fn over(repo: git2::Repository, city_root: &Path) -> Worktrees {
        Worktrees {
            repo,
            home: city_root.join(kernel::RESERVED_PREFIX).join(WORKTREE_DIR),
            city_root: city_root.to_path_buf(),
            ceiling: ByteLen::new(WORKTREE_MAX_BYTES),
        }
    }

    /// Lends one node its tree: the one it was given before when that
    /// tree is still on disk, reset to the node's branch, and a new one
    /// otherwise.
    ///
    /// # Errors
    /// Refuses a tree somebody holds, a city whose working tree exceeds
    /// the ceiling, and a repository with no commit to branch from.
    pub fn claim(
        &self,
        name: &WorktreeName,
        scopes: &[String],
    ) -> Result<WorktreeLease, StorageError> {
        match self.standing(name)? {
            Standing::Held => Err(StorageError::WorktreeBusy {
                name: name.as_str().to_owned(),
                detail: "another node holds this tree".to_owned(),
            }),
            Standing::Kept(tree) => self.reattach(name, &tree, scopes),
            Standing::Absent => self.place(name),
        }
    }

    /// Places a new tree for `name`, locked from the moment it exists:
    /// the stock taken over when there is one to take, and a checkout of
    /// the whole tree otherwise (`crates/storage/spec/Worktree/Trees/Stock.lean` §8-35).
    fn place(&self, name: &WorktreeName) -> Result<WorktreeLease, StorageError> {
        let city = self.refuse_oversized(name.as_str())?;
        if self.repo.head().is_err() {
            return Err(StorageError::Worktree {
                op: "branch a worktree",
                detail: "the city has no checkpoint yet, and a tree branches from a commit"
                    .to_owned(),
            });
        }
        // `standing` found no registration, so a directory under this
        // name is what a placement that failed halfway left behind.
        self.clear_unregistered(name.as_str())?;
        if let Some(lease) = self.adopt(name, &city)? {
            return Ok(lease);
        }
        // A node that has held a tree before still has its branch: the
        // tree is a materialization, the branch is the line of work.
        // Reattaching is what makes releasing a tree cheap enough to do
        // between sessions.
        let branch = match self
            .repo
            .find_branch(name.as_str(), git2::BranchType::Local)
        {
            Ok(branch) => Some(branch),
            Err(err) if err.code() == git2::ErrorCode::NotFound => None,
            Err(err) => {
                return Err(StorageError::Worktree {
                    op: "find a node branch",
                    detail: format!("{}: {err}", name.as_str()),
                });
            }
        };
        self.add_tree(name, branch.as_ref().map(git2::Branch::get), &city)
    }

    /// Refuses a tree before it exists when the city working tree it
    /// copies is over the ceiling, and otherwise answers what that
    /// working tree weighed. `id` names the tree in the refusal.
    pub(super) fn refuse_oversized(&self, id: &str) -> Result<Weight, StorageError> {
        let source = self.repo.workdir().ok_or_else(|| StorageError::Worktree {
            op: "find the city working tree",
            detail: "the repository is bare".to_owned(),
        })?;
        let city = measure(source)?;
        if city.bytes.get() > self.ceiling.get() {
            return Err(StorageError::WorktreeBusy {
                name: id.to_owned(),
                detail: format!(
                    "the city working tree is {} bytes and the ceiling is {}",
                    city.bytes.get(),
                    self.ceiling.get()
                ),
            });
        }
        Ok(city)
    }

    /// Checks out the tree for `name` on `reference` when one is given
    /// and on a new branch from the trunk otherwise. `city` is the walk
    /// the ceiling was judged on, counted into the lease's work.
    pub(super) fn add_tree(
        &self,
        name: &WorktreeName,
        reference: Option<&git2::Reference<'_>>,
        city: &Weight,
    ) -> Result<WorktreeLease, StorageError> {
        let (path, checked_out) = self.check_out_tree(name.as_str(), reference)?;
        Ok(WorktreeLease {
            name: name.clone(),
            disk: checked_out.bytes,
            path,
            work: FileWork {
                created: checked_out.files,
                walked: city.walked,
                ..FileWork::default()
            },
        })
    }

    /// Registers the tree `id` under the tree home, locked, and checks it
    /// out whole; answers where it is and what the checkout wrote.
    pub(super) fn check_out_tree(
        &self,
        id: &str,
        reference: Option<&git2::Reference<'_>>,
    ) -> Result<(PathBuf, Written), StorageError> {
        std::fs::create_dir_all(&self.home).map_err(|source| StorageError::Io {
            op: "create the worktree home",
            path: self.home.clone(),
            source,
        })?;
        let path = self.home.join(id);
        // The checkout lands at this name, and a name that is an alias
        // would write the whole tree through it (`crates/storage/spec/Alias.lean` §8-25).
        crate::alias::WriteTarget::within("place a worktree", &self.city_root, &path)?;
        let mut opts = git2::WorktreeAddOptions::new();
        opts.lock(true);
        opts.reference(reference);
        let tree =
            self.repo
                .worktree(id, &path, Some(&opts))
                .map_err(|err| StorageError::Worktree {
                    op: "add a worktree",
                    detail: format!("{id}: {err}"),
                })?;
        // The size is read from the index the checkout just wrote rather
        // than by walking the tree it wrote (`crates/storage/spec/Worktree.lean` §8-31).
        Ok((path, written(&tree)?))
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
    pub fn plan_merge(&self, name: &WorktreeName) -> Result<PlannedMerge<'_>, StorageError> {
        let refuse = |op: &'static str, detail: String| StorageError::Worktree { op, detail };
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
            return Err(StorageError::MergeStale {
                name: name.as_str().to_owned(),
                detail: format!("the trunk moved to {} after this node branched", ours.id()),
            });
        }
        let tree = theirs
            .tree()
            .map_err(|err| refuse("read the node tree", err.to_string()))?;
        check_out(&self.repo, &tree, CheckoutRun::DryRun)?;
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
    ) -> Result<(), StorageError> {
        let refuse = |op: &'static str, detail: String| StorageError::Worktree { op, detail };
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
            landing.reviewer(&self.city_root)
        );
        let merge = self
            .repo
            .commit(
                None,
                &signature,
                &signature,
                &message,
                &tree,
                &[&ours, &theirs],
            )
            .map_err(|err| refuse("commit the merge", err.to_string()))?;
        check_out(&self.repo, &tree, CheckoutRun::Write)?;
        let trunk = head.name().unwrap_or("HEAD");
        self.repo
            .reference_matching(trunk, merge, true, ours.id(), landing.subject)
            .map_err(|err| refuse("move the city trunk", err.to_string()))?;
        Ok(())
    }

    /// Gives a tree back and keeps its files for the node's next run:
    /// the lock is the lease, so lifting it is the whole release.
    ///
    /// # Errors
    /// Propagates a repository that cannot find or unlock the tree.
    pub fn release(&self, lease: WorktreeLease) -> Result<(), StorageError> {
        let refuse = |op: &'static str, err: git2::Error| StorageError::Worktree {
            op,
            detail: format!("{}: {err}", lease.name().as_str()),
        };
        self.repo
            .find_worktree(lease.name().as_str())
            .map_err(|err| refuse("find a worktree", err))?
            .unlock()
            .map_err(|err| refuse("unlock a worktree", err))
    }

    /// Every node's tree the repository knows about, sorted. The stock
    /// is no node's tree, and is not among them.
    ///
    /// # Errors
    /// Propagates a repository that cannot list its worktrees.
    pub fn live(&self) -> Result<Vec<WorktreeName>, StorageError> {
        let names = self
            .repo
            .worktrees()
            .map_err(|err| StorageError::Worktree {
                op: "list worktrees",
                detail: err.to_string(),
            })?;
        let mut out = Vec::new();
        // A name git cannot render as UTF-8 was not written by this
        // module, and it is not a tree this city can address.
        for name in names.iter().flatten().flatten() {
            if name != stock::STOCK {
                out.push(WorktreeName::parse(name)?);
            }
        }
        out.sort();
        Ok(out)
    }
}

/// Opens the city's repository with only its own config file read.
pub(super) fn open_city(city_root: &Path) -> Result<git2::Repository, git2::Error> {
    own_config(git2::Repository::open(city_root)?)
}

/// Opens the repository of one of the city's trees the same way.
pub(super) fn open_tree(tree: &git2::Worktree) -> Result<git2::Repository, git2::Error> {
    own_config(git2::Repository::open_from_worktree(tree)?)
}

// A User's global, XDG or system file can vanish between libgit2's look and
// its stat (`crates/storage/spec/Worktree.lean` §8-9), so no later call
// reads them.
fn own_config(repo: git2::Repository) -> Result<git2::Repository, git2::Error> {
    let mut config = git2::Config::new()?;
    config.add_file(
        &repo.commondir().join("config"),
        git2::ConfigLevel::Local,
        false,
    )?;
    repo.set_config(&config)?;
    Ok(repo)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
pub(crate) mod tests;
