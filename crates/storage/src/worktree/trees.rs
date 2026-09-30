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
use super::weight::{Weight, measure, written};
use kept::Standing;

mod kept;

/// Where the trees live: inside the reserved subtree, because they are
/// the city's own machinery rather than anybody's writable space. What a
/// run may write is judged against the tree it works in.
pub(super) const WORKTREE_DIR: &str = "worktrees";

/// The city's trees.
pub struct Worktrees {
    pub(super) repo: git2::Repository,
    pub(super) home: PathBuf,
    /// Where the person placed the city; the alias check stops here.
    city_root: PathBuf,
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
        let repo = git2::Repository::open(city_root).map_err(|err| StorageError::Worktree {
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

    /// Places a new tree for `name`, locked from the moment it exists.
    fn place(&self, name: &WorktreeName) -> Result<WorktreeLease, StorageError> {
        let city = self.refuse_oversized(name)?;
        if self.repo.head().is_err() {
            return Err(StorageError::Worktree {
                op: "branch a worktree",
                detail: "the city has no checkpoint yet, and a tree branches from a commit"
                    .to_owned(),
            });
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
    /// working tree weighed.
    pub(super) fn refuse_oversized(&self, name: &WorktreeName) -> Result<Weight, StorageError> {
        let source = self.repo.workdir().ok_or_else(|| StorageError::Worktree {
            op: "find the city working tree",
            detail: "the repository is bare".to_owned(),
        })?;
        let city = measure(source)?;
        if city.bytes.get() > self.ceiling.get() {
            return Err(StorageError::WorktreeBusy {
                name: name.as_str().to_owned(),
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
        std::fs::create_dir_all(&self.home).map_err(|source| StorageError::Io {
            op: "create the worktree home",
            path: self.home.clone(),
            source,
        })?;
        let path = self.home.join(name.as_str());
        // The checkout lands at this name, and a name that is an alias
        // would write the whole tree through it (storage-SPEC 8-25).
        crate::alias::WriteTarget::within("place a worktree", &self.city_root, &path)?;
        let mut opts = git2::WorktreeAddOptions::new();
        opts.lock(true);
        opts.reference(reference);
        let tree = self
            .repo
            .worktree(name.as_str(), &path, Some(&opts))
            .map_err(|err| StorageError::Worktree {
                op: "add a worktree",
                detail: format!("{}: {err}", name.as_str()),
            })?;
        let checked_out = written(&tree)?;
        let weight = measure(&path)?;
        Ok(WorktreeLease {
            name: name.clone(),
            disk: weight.bytes,
            path,
            work: FileWork {
                created: checked_out.files,
                walked: city.walked.saturating_add(weight.walked),
                ..FileWork::default()
            },
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
            self.reviewer(landing.reviewed_by_person)
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

    /// Unregisters `name`, taking its files with it where git will.
    ///
    /// A repository that has already forgotten the tree is the end
    /// state this asks for, so it is not a failure.
    pub(super) fn forget(&self, name: &WorktreeName) -> Result<(), StorageError> {
        let tree = match self.repo.find_worktree(name.as_str()) {
            Ok(tree) => tree,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
            Err(err) => {
                return Err(StorageError::Worktree {
                    op: "find a worktree",
                    detail: format!("{}: {err}", name.as_str()),
                });
            }
        };
        let mut opts = git2::WorktreePruneOptions::new();
        // A lock on a tree whose directory is gone guards nothing.
        opts.valid(true).locked(true).working_tree(true);
        tree.prune(Some(&mut opts))
            .map_err(|err| StorageError::Worktree {
                op: "prune a worktree",
                detail: format!("{}: {err}", name.as_str()),
            })
    }

    /// Every tree the repository knows about, sorted.
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
