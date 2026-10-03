// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A building's base checkpoint, written as one pack.

use std::io::Write;

use kernel::{Payload, TimeMs};

use crate::error::StorageError;

use super::commit::{Checkpoint, checkpoint_ref, committed, git_err, subject_of};
use super::opening::{HeadMove, IndexOwner, commit_refused};
use super::provenance::Provenance;
use super::scan::CommitPlan;

/// How far a base checkpoint has got, reported as each step lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseProgress {
    /// The building's files are in the index and their blobs in memory.
    Staged { files: usize },
    /// The one pack holding every object the checkpoint wrote is on disk.
    Packed { bytes: usize },
}

/// What one base checkpoint stages and how it signs: three values a caller
/// never passes apart.
pub(super) struct BaseOf<'a> {
    pub(super) scopes: &'a [String],
    pub(super) t: TimeMs,
    pub(super) of: &'a Provenance,
}

/// Where a base checkpoint's commit is filed, decided under `HEAD_MOVES`.
pub(super) enum BaseTarget {
    /// The city has no commit: the commit becomes the branch HEAD names,
    /// created only if that branch is still absent.
    Head,
    /// The city has history: the commit is filed like a wave checkpoint.
    Filed,
}

impl Checkpoint {
    /// Stages `scopes`, scans them and commits, with every object held
    /// in memory until the end and then written as one pack.
    ///
    /// Takes the handle by value: the in-memory store is installed on
    /// this handle's object database and cannot be taken off again, so
    /// the handle goes with it, and no later checkpoint can write an object
    /// into a store nobody dumps (`crates/storage/spec/Checkpoint.lean` §8-8).
    ///
    /// The commit moves HEAD when the city has none, under `HEAD_MOVES` and
    /// by compare-and-swap (storage D27), and is otherwise filed under
    /// `refs/sprawling/runs/<run>/<oid>` like a wave checkpoint.
    ///
    /// # Errors
    /// Propagates staging, the staged-secret scan, the commit, a pack the
    /// object database refuses, a reference it will not write, and a HEAD
    /// another writer created after this one read it as absent.
    pub fn base_checkpoint(
        self,
        scopes: &[String],
        t: TimeMs,
        of: &Provenance,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Payload, StorageError> {
        if Self::head_commit(&self.repo)?.is_some() {
            return self.write_held(&BaseOf { scopes, t, of }, BaseTarget::Filed, progress);
        }
        let _head_moves = super::opening::moving_head()?;
        let target = match Self::head_commit(&self.repo)? {
            None => BaseTarget::Head,
            Some(_) => BaseTarget::Filed,
        };
        self.write_held(&BaseOf { scopes, t, of }, target, progress)
    }

    /// A second handle on the same repository and the same index file, for
    /// a caller that keeps its own handle and still writes one base pack.
    ///
    /// # Errors
    /// A repository or a writer's index that does not open.
    pub(super) fn reopened(&self) -> Result<Checkpoint, StorageError> {
        let repo = git2::Repository::open(self.repo.path())
            .map_err(git_err("open the city repository for a base"))?;
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
        })
    }

    /// The mempack write itself; the caller decided `target` under
    /// `HEAD_MOVES` when it is [`BaseTarget::Head`].
    pub(super) fn write_held(
        mut self,
        base: &BaseOf<'_>,
        target: BaseTarget,
        progress: &mut dyn FnMut(BaseProgress),
    ) -> Result<Payload, StorageError> {
        let objects = self.repo.path().join("objects");
        let objects = objects.to_str().ok_or_else(|| StorageError::Checkpoint {
            op: "open the object store",
            detail: format!("{} is not valid UTF-8", objects.display()),
        })?;
        // The disk store is an alternate, and libgit2 writes to no
        // alternate: every object this checkpoint makes lands in memory.
        let odb = git2::Odb::new().map_err(git_err("open an in-memory object store"))?;
        let held = odb
            .add_new_mempack_backend(1)
            .map_err(git_err("open an in-memory object store"))?;
        odb.add_disk_alternate(objects)
            .map_err(git_err("read the object store"))?;
        self.repo
            .set_odb(&odb)
            .map_err(git_err("hold the base checkpoint in memory"))?;

        let BaseOf { scopes, t, of } = *base;
        let files = self.stage_scopes_held(scopes)?;
        progress(BaseProgress::Staged { files: files.len() });
        self.scan_staged()?;
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

        // Only now are the objects on disk, so only now may the index and
        // a reference name them: a failure above leaves both as they were.
        super::scan::write_index(&mut self.repo.index().map_err(git_err("read index"))?)?;
        match target {
            // Created only while absent: the swap from the "no HEAD" this
            // writer read (`a_held_base_moves_head_only_from_what_it_read`).
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
        committed(oid, of, scopes, files)
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
