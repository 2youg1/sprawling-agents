// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::TimeMs;
use kernel::event::record::Commit;

use crate::StorageError;

use super::commit::{Checkpoint, CommitDetails, commit_record, git_err};
use super::opening::HeadMove;
use super::provenance::Provenance;
use super::scan::CommitPlan;

impl Checkpoint {
    pub fn land(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        subject: &str,
    ) -> Result<Commit, StorageError> {
        let _head_moves = super::opening::moving_head(self.head_lock())?;
        let files = self.stage_scopes(scopes)?;

        self.land_staged(
            scopes,
            &CommitPlan {
                t,
                of,
                subject,
                head: HeadMove::Advance,
            },
            files,
        )
    }

    pub fn land_changes(
        &mut self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        subject: &str,
    ) -> Result<Option<Commit>, StorageError> {
        let _head_moves = super::opening::moving_head(self.head_lock())?;
        let files = self.stage_scopes(scopes)?;
        let parent = Self::parent_at(&self.repo, self.reference.as_deref())?;
        let staged = self
            .repo
            .index()
            .and_then(|mut index| index.write_tree())
            .map_err(git_err("compare completed work"))?;
        if parent
            .as_ref()
            .is_some_and(|commit| commit.tree_id() == staged)
        {
            return Ok(None);
        }
        drop(parent);
        self.land_staged(
            scopes,
            &CommitPlan {
                t,
                of,
                subject,
                head: HeadMove::Advance,
            },
            files,
        )
        .map(Some)
    }

    fn land_staged(
        &mut self,
        scopes: &[String],
        plan: &CommitPlan<'_>,
        files: Vec<String>,
    ) -> Result<Commit, StorageError> {
        let withheld = self.scan_staged(scopes)?;
        let oid = self.commit(plan)?;
        self.repo
            .reference(
                &super::commit::checkpoint_ref(plan.of, oid),
                oid,
                true,
                "pin the reviewed candidate",
            )
            .map_err(git_err("pin the reviewed candidate"))?;
        commit_record(
            oid,
            CommitDetails {
                of: plan.of,
                scopes,
                files,
                withheld,
            },
        )
    }
}
