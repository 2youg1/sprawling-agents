// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;

use kernel::TimeMs;
use kernel::event::record::Commit;

use crate::error::StorageError;

use super::commit::{
    Checkpoint, CommitDetails, checkpoint_ref, commit_record, git_err, subject_of,
};
use super::opening::{HeadMove, IndexOwner, commit_refused};
use super::provenance::Provenance;
use super::scan::CommitPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseProgress {
    Staged { files: usize },

    Packed { bytes: usize },
}

pub(crate) struct BaseOf<'a> {
    pub(crate) scopes: &'a [String],
    pub(crate) t: TimeMs,
    pub(crate) of: &'a Provenance,
    pub(crate) source: Option<&'a crate::SourceSnapshot>,
}

pub(super) enum BaseTarget {
    Head,

    Filed,
}

impl Checkpoint {
    pub fn base_checkpoint(
        self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Commit, StorageError> {
        self.import_base(
            &BaseOf {
                scopes,
                t,
                of,
                source: None,
            },
            progress,
        )
    }

    pub(crate) fn import_base(
        self,
        base: &BaseOf<'_>,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Commit, StorageError> {
        if Self::head_commit(&self.repo)?.is_some() {
            return self.write_held(base, BaseTarget::Filed, progress);
        }
        let _head_moves = super::opening::moving_head(self.repo.path())?;
        let target = match Self::head_commit(&self.repo)? {
            None => BaseTarget::Head,
            Some(_) => BaseTarget::Filed,
        };
        self.write_held(base, target, progress)
    }

    pub(super) fn reopened(&self) -> Result<Checkpoint, StorageError> {
        let repo = git2::Repository::open(self.repo.path())
            .map_err(git_err("open the city repository for a base"))?;
        if let Some(workdir) = self.repo.workdir() {
            repo.set_workdir(workdir, false)
                .map_err(git_err("attach the project working directory"))?;
        }
        let index = match &self.index {
            IndexOwner::City => IndexOwner::City,
            IndexOwner::Writer(own) => {
                let mut index = git2::Index::open(own).map_err(git_err("open a writer's index"))?;
                repo.set_index(&mut index)
                    .map_err(git_err("give a writer its index"))?;
                IndexOwner::Writer(own.clone())
            }
        };
        Ok(Checkpoint {
            repo,
            last: None,
            index,
            reference: self.reference.clone(),
            entropy_allow: self.entropy_allow.clone(),
        })
    }

    pub(super) fn write_held(
        mut self,
        base: &BaseOf<'_>,
        target: BaseTarget,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Commit, StorageError> {
        let objects = self.repo.path().join("objects");
        let objects = objects.to_str().ok_or_else(|| StorageError::Checkpoint {
            op: "open the object store",
            detail: format!("{} is not valid UTF-8", objects.display()),
        })?;

        let odb = git2::Odb::new().map_err(git_err("open an in-memory object store"))?;
        let held = odb
            .add_new_mempack_backend(1)
            .map_err(git_err("open an in-memory object store"))?;
        odb.add_disk_alternate(objects)
            .map_err(git_err("read the object store"))?;
        self.repo
            .set_odb(&odb)
            .map_err(git_err("hold the base checkpoint in memory"))?;

        let BaseOf {
            scopes,
            t,
            of,
            source,
        } = *base;
        let mut files = self.stage_scopes_held(scopes)?;
        if let Some(source) = source {
            files.extend(source.stage(&self.repo, of.actor())?);
            files.sort_unstable();
            files.dedup();
        }
        progress(BaseProgress::Staged { files: files.len() });
        let withheld = self.scan_staged(scopes)?;
        let oid = self.commit(&CommitPlan {
            t,
            of,
            subject: &subject_of(scopes),
            head: HeadMove::Leave,
        })?;

        let mut pack = git2::Buf::new();
        held.dump(&self.repo, &mut pack)
            .map_err(git_err("dump the base checkpoint"))?;
        let disk =
            git2::Repository::open(self.repo.path()).map_err(git_err("open the object store"))?;
        let disk_odb = disk.odb().map_err(git_err("open the object store"))?;
        let mut writer = disk_odb
            .packwriter()
            .map_err(git_err("write the base checkpoint pack"))?;
        writer
            .write_all(&pack)
            .map_err(|err| StorageError::Checkpoint {
                op: "write the base checkpoint pack",
                detail: err.to_string(),
            })?;
        writer
            .commit()
            .map_err(git_err("write the base checkpoint pack"))?;
        progress(BaseProgress::Packed { bytes: pack.len() });

        super::scan::write_index(&mut self.repo.index().map_err(git_err("read index"))?)?;
        match target {
            BaseTarget::Head => {
                let branch = self
                    .repo
                    .find_reference("HEAD")
                    .map_err(git_err("read HEAD"))?
                    .symbolic_target()
                    .map_err(|err| StorageError::Checkpoint {
                        op: "read HEAD",
                        detail: err.to_string(),
                    })?
                    .map(str::to_owned)
                    .ok_or_else(|| StorageError::Checkpoint {
                        op: "move HEAD to the base checkpoint",
                        detail: "HEAD names no branch".to_owned(),
                    })?;
                self.repo
                    .reference(&branch, oid, false, "sprawling: a base checkpoint")
                    .map_err(|err| commit_refused(Some("HEAD"), err))?;
            }
            BaseTarget::Filed => {
                self.repo
                    .reference(
                        &checkpoint_ref(of, oid),
                        oid,
                        true,
                        "sprawling: a base checkpoint",
                    )
                    .map_err(git_err("file a base checkpoint"))?;
            }
        }
        held.reset()
            .map_err(git_err("release the base checkpoint"))?;
        commit_record(
            oid,
            CommitDetails {
                of,
                scopes,
                files,
                withheld,
            },
        )
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
mod tests;
