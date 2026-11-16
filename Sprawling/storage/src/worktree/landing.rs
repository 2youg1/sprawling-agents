// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::TimeMs;

use crate::checkpoint::Provenance;
use crate::error::StorageError;

use super::trees::Worktrees;

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

pub struct PlannedMerge<'a> {
    pub(super) trees: &'a Worktrees,
    pub(super) target: git2::Oid,
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

    pub fn apply(self, landing: &Landing<'_>) -> Result<(), StorageError> {
        self.trees.land_merge(self.target, landing)
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
