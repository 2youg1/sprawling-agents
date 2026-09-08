// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Worktrees: claim, merge, release.

use std::path::{Path, PathBuf};

use kernel::{ByteLen, TimeMs, consts_policy::WORKTREE_MAX_BYTES};

use crate::checkpoint::Provenance;
use crate::error::MemoryError;

use super::lease::WorktreeLease;
use super::name::WorktreeName;

/// What one merge writes down: four values that arrive together and are
/// meaningless apart - a subject with nobody's provenance under it says
/// who merged nothing. `reviewed_by_person` adds one `Reviewed-by: Name
/// <email>` trailer, and only when this repository's git config also
/// carries `user.name` and `user.email`: the city does not invent a
/// person's name.
pub struct Landing<'a> {
    /// Determinism rule 2: the signature carries the injected instant.
    pub t: TimeMs,
    pub of: &'a Provenance,
    pub subject: &'a str,
    pub reviewed_by_person: bool,
}

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

/// A merge that has been decided and not yet made.
///
/// The commit the trunk will land on is settled at construction and
/// every refusal has already happened, so the line announcing this merge
/// can be written before the trunk moves. [`PlannedMerge::apply`] is the
/// only way to move it, and [`Worktrees::plan_merge`] is this value's
/// only source.
pub struct PlannedMerge<'a> {
    trees: &'a Worktrees,
    target: git2::Oid,
}

/// Names the decision, not the repository holding it: a `Worktrees` has
/// no useful `Debug` and printing one would say nothing about which
/// merge this is.
impl std::fmt::Debug for PlannedMerge<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannedMerge")
            .field("commit", &self.target)
            .finish()
    }
}

impl PlannedMerge<'_> {
    /// The commit the city trunk will point at, for the line that says so.
    pub fn commit(&self) -> String {
        self.target.to_string()
    }

    /// Brings a node's committed work into the city's own trunk, as a
    /// merge commit carrying the merging run's trailers. The judgement
    /// stays fast-forward only; what changed in card-2.3 is what the
    /// history keeps, because a pointer move leaves nothing to read and
    /// no place to say who verified it.
    ///
    /// # Errors
    /// Propagates a trunk that cannot be read, committed onto or checked
    /// out. The fast-forward judgement is not repeated: it was made, and
    /// refused if it had to be, before this value existed.
    pub fn apply(self, landing: &Landing<'_>) -> Result<(), MemoryError> {
        self.trees.land_merge(self.target, landing)
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
        if self.live()?.contains(name) {
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
        if theirs.id() != ours.id()
            && !self
                .repo
                .graph_descendant_of(theirs.id(), ours.id())
                .unwrap_or(false)
        {
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
    fn land_merge(&self, target: git2::Oid, landing: &Landing<'_>) -> Result<(), MemoryError> {
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

    /// Gives a tree back: the files go, then the repository forgets it.
    ///
    /// # Errors
    /// Propagates a directory that cannot be removed and a repository
    /// that refuses to prune.
    pub fn release(&self, lease: WorktreeLease) -> Result<(), MemoryError> {
        if lease.path().exists() {
            std::fs::remove_dir_all(lease.path()).map_err(|source| MemoryError::Io {
                op: "remove a worktree",
                path: lease.path().to_path_buf(),
                source,
            })?;
        }
        let tree = self
            .repo
            .find_worktree(lease.name().as_str())
            .map_err(|err| MemoryError::Worktree {
                op: "find a worktree",
                detail: format!("{}: {err}", lease.name().as_str()),
            })?;
        let mut opts = git2::WorktreePruneOptions::new();
        opts.valid(true).working_tree(true);
        tree.prune(Some(&mut opts))
            .map_err(|err| MemoryError::Worktree {
                op: "prune a worktree",
                detail: format!("{}: {err}", lease.name().as_str()),
            })
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

/// Bytes under a directory, git's own bookkeeping excluded. An explicit
/// worklist rather than recursion: a deep tree is a data-dependent depth,
/// and a stack overflow is not a failure a caller can handle.
fn measure(root: &Path) -> Result<ByteLen, MemoryError> {
    let mut total: u64 = 0;
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(MemoryError::Io {
                    op: "measure a worktree",
                    path: dir.clone(),
                    source,
                });
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue; // a link's target is measured where it lives
            }
            if kind.is_dir() {
                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }
                pending.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Ok(ByteLen::new(total))
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
