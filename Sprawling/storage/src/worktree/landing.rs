// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::TimeMs;

use crate::checkpoint::Provenance;
use crate::error::StorageError;

use super::trees::Worktrees;

pub(super) const PENDING: &str = "refs/sprawling/pending";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LandingRecord {
    Absent,
    Completed,
    Reviewed,
}

impl Worktrees {
    pub fn reconcile_landing(
        &self,
        recorded: impl FnOnce(kernel::GitOid) -> Result<LandingRecord, kernel::AxError>,
    ) -> Result<(), StorageError> {
        let _moving = crate::checkpoint::opening::moving_head(self.repo.path())?;
        let pending = match self.repo.find_reference(PENDING) {
            Ok(reference) => reference,
            Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(()),
            Err(err) => return Err(landing_error(err)),
        };
        let oid = pending.peel_to_commit().map_err(landing_error)?.id();
        let point = kernel::GitOid::parse(&oid.to_string()).ok_or_else(|| {
            landing_error(git2::Error::from_str("invalid pending merge identifier"))
        })?;
        match recorded(point).map_err(|source| StorageError::Draft { source })? {
            LandingRecord::Absent => self
                .repo
                .find_reference(PENDING)
                .and_then(|mut reference| reference.delete())
                .map_err(landing_error),
            LandingRecord::Completed => self.complete_landing(oid, false),
            LandingRecord::Reviewed => self.complete_landing(oid, true),
        }
    }

    pub(super) fn complete_landing(
        &self,
        oid: git2::Oid,
        reviewed: bool,
    ) -> Result<(), StorageError> {
        let repo = &self.repo;
        let merge = repo.find_commit(oid).map_err(landing_error)?;
        let base = merge.parent_id(0).map_err(landing_error)?;
        let head = repo.head().map_err(landing_error)?;
        let current = head.peel_to_commit().map_err(landing_error)?.id();
        if current == base {
            let tree = merge.tree().map_err(landing_error)?;
            check_out(repo, &tree, CheckoutRun::Write)?;
            repo.reference_matching(
                head.name().map_err(landing_error)?,
                oid,
                true,
                base,
                "complete project landing",
            )
            .map_err(landing_error)?;
        } else if current != oid
            && !repo
                .graph_descendant_of(current, oid)
                .map_err(landing_error)?
        {
            return Err(StorageError::MergeStale {
                name: head.name().map_err(landing_error)?.to_owned(),
                detail: format!(
                    "the recorded merge {oid} expects {base}, but the project is at {current}"
                ),
            });
        }
        if reviewed {
            repo.reference(
                &format!("refs/sprawling/reviewed/{oid}"),
                oid,
                true,
                "pin the reviewed merge",
            )
            .map_err(landing_error)?;
        }
        repo.find_reference(PENDING)
            .and_then(|mut pending| pending.delete())
            .map_err(landing_error)
    }
}

fn landing_error(err: git2::Error) -> StorageError {
    StorageError::Worktree {
        op: "finish the recorded project landing",
        detail: err.to_string(),
    }
}

pub struct Landing<'a> {
    pub t: TimeMs,
    pub of: &'a Provenance,
    pub subject: &'a str,
    pub reviewed_by_person: bool,
}

impl Landing<'_> {
    pub(super) fn reviewer(&self, city_root: &Path) -> String {
        if !self.reviewed_by_person {
            return String::new();
        }

        let Ok(config) = git2::Repository::open(city_root).and_then(|repo| repo.config()) else {
            return String::new();
        };
        match (
            config.get_string("user.name"),
            config.get_string("user.email"),
        ) {
            (Ok(name), Ok(email)) => format!(
                "Reviewed-by: {name} <{email}>
"
            ),
            _ => String::new(),
        }
    }
}

pub enum ReviewPlan<'a> {
    Ready(PlannedMerge<'a>),
    Renewed(kernel::GitOid),
    Conflicted,
}

impl Worktrees {
    pub fn prepare_review(
        &self,
        name: &super::name::WorktreeName,
        point: &kernel::GitOid,
    ) -> Result<ReviewPlan<'_>, StorageError> {
        match self.plan_reviewed(name, point) {
            Ok(plan) => return Ok(ReviewPlan::Ready(plan)),
            Err(StorageError::MergeStale { .. }) => {}
            Err(error) => return Err(error),
        };
        let offered = self.commit_at(point)?;
        let trunk = self
            .repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(landing_error)?;
        let mut index = self
            .repo
            .merge_commits(&trunk, &offered, None)
            .map_err(landing_error)?;
        if index.has_conflicts() {
            return Ok(ReviewPlan::Conflicted);
        }
        let tree = self
            .repo
            .find_tree(index.write_tree_to(&self.repo).map_err(landing_error)?)
            .map_err(landing_error)?;
        let renewed = self
            .repo
            .commit(
                None,
                &offered.author(),
                &offered.committer(),
                offered.message().map_err(landing_error)?,
                &tree,
                &[&trunk, &offered],
            )
            .map_err(landing_error)?;
        self.repo
            .reference(
                &format!("refs/sprawling/reviews/{renewed}"),
                renewed,
                true,
                "pin the renewed review candidate",
            )
            .map_err(landing_error)?;
        kernel::GitOid::parse(&renewed.to_string())
            .map(ReviewPlan::Renewed)
            .ok_or_else(|| {
                landing_error(git2::Error::from_str("invalid renewed review identifier"))
            })
    }
}

pub struct PlannedMerge<'a> {
    pub(super) trees: &'a Worktrees,
    pub(super) trunk: String,
    pub(super) base: git2::Oid,
    pub(super) target: git2::Oid,
    pub(super) tree: git2::Oid,
    pub(super) reviewed: bool,
}

impl std::fmt::Debug for PlannedMerge<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannedMerge")
            .field("commit", &self.target)
            .finish()
    }
}

impl PlannedMerge<'_> {
    pub fn commit(&self) -> String {
        self.target.to_string()
    }

    pub fn apply(self, landing: &Landing<'_>) -> Result<String, StorageError> {
        self.before_landing(landing, |_| Ok(()))
    }

    pub fn before_landing(
        self,
        landing: &Landing<'_>,
        record: impl FnOnce(String) -> Result<(), kernel::AxError>,
    ) -> Result<String, StorageError> {
        self.trees.land_merge(&self, landing, record)
    }
}

#[derive(Clone, Copy)]
pub(super) enum CheckoutRun {
    DryRun,
    Write,
}

pub(super) fn check_out(
    repo: &git2::Repository,
    tree: &git2::Tree<'_>,
    run: CheckoutRun,
) -> Result<(), StorageError> {
    let mut paths = Vec::new();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe();
    match run {
        CheckoutRun::DryRun => checkout.dry_run(),
        CheckoutRun::Write => &mut checkout,
    };
    checkout.notify_on(git2::CheckoutNotificationType::CONFLICT);
    checkout.notify(|_, path, _, _, _| {
        paths.extend(path.map(|p| p.to_string_lossy().into_owned()));
        true
    });
    let outcome = repo.checkout_tree(tree.as_object(), Some(&mut checkout));
    drop(checkout);
    if !paths.is_empty() {
        return Err(StorageError::MergeWouldDiscard { paths });
    }
    outcome.map_err(|err| StorageError::Worktree {
        op: "check out the merged trunk",
        detail: err.to_string(),
    })
}
