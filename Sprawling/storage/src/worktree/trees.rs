// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(super) const WORKTREE_DIR: &str = "worktrees";

pub struct Worktrees {
    pub(super) repo: git2::Repository,
    pub(super) home: PathBuf,

    pub(super) city_root: PathBuf,
    ceiling: ByteLen,
}

impl std::fmt::Debug for Worktrees {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worktrees")
            .field("home", &self.home)
            .field("ceiling", &self.ceiling)
            .finish()
    }
}

impl Worktrees {

    pub fn open(city_root: &Path) -> Result<Worktrees, StorageError> {
        let repo = open_city(city_root).map_err(|err| StorageError::Worktree {
            op: "open the city repository",
            detail: format!("{}: {err}", city_root.display()),
        })?;
        Ok(Worktrees::over(repo, city_root))
    }

    pub(super) fn over(repo: git2::Repository, city_root: &Path) -> Worktrees {
        Worktrees {
            repo,
            home: city_root.join(kernel::RESERVED_PREFIX).join(WORKTREE_DIR),
            city_root: city_root.to_path_buf(),
            ceiling: ByteLen::new(WORKTREE_MAX_BYTES),
        }
    }

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

    fn place(&self, name: &WorktreeName) -> Result<WorktreeLease, StorageError> {
        let city = self.refuse_oversized(name.as_str())?;
        if self.repo.head().is_err() {
            return Err(StorageError::Worktree {
                op: "branch a worktree",
                detail: "the city has no checkpoint yet, and a tree branches from a commit"
                    .to_owned(),
            });
        }

        self.clear_unregistered(name.as_str())?;
        if let Some(lease) = self.adopt(name, &city)? {
            return Ok(lease);
        }

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

        Ok((path, written(&tree)?))
    }

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

    pub fn live(&self) -> Result<Vec<WorktreeName>, StorageError> {
        let names = self
            .repo
            .worktrees()
            .map_err(|err| StorageError::Worktree {
                op: "list worktrees",
                detail: err.to_string(),
            })?;
        let mut out = Vec::new();

        for name in names.iter().flatten().flatten() {
            if name != stock::STOCK {
                out.push(WorktreeName::parse(name)?);
            }
        }
        out.sort();
        Ok(out)
    }
}

pub(super) fn open_city(city_root: &Path) -> Result<git2::Repository, git2::Error> {
    own_config(git2::Repository::open(city_root)?)
}

pub(super) fn open_tree(tree: &git2::Worktree) -> Result<git2::Repository, git2::Error> {
    own_config(git2::Repository::open_from_worktree(tree)?)
}

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
